import { useState, type ReactNode } from "react";
import { useForm } from "@tanstack/react-form";
import { useQuery } from "@tanstack/react-query";
import { DndContext, PointerSensor, closestCenter, useSensor, useSensors, type DragEndEvent } from "@dnd-kit/core";
import { SortableContext, useSortable, verticalListSortingStrategy } from "@dnd-kit/sortable";
import { CSS } from "@dnd-kit/utilities";
import { IconGripVertical, IconPlus, IconTrash } from "@tabler/icons-react";
import { modals, type ContextModalProps } from "@mantine/modals";
import { ActionIcon, Alert, Button, ColorSwatch, Group, Loader, Stack, Switch, Text, TextInput, UnstyledButton } from "@mantine/core";
import { z } from "zod";
import { tracksQueryOptions } from "../queries";
import { useSaveTracks } from "../mutations";
import { ALL_DAYS, TRACK_COLORS, makeTrackKey } from "../trackSettings";
import type { Track } from "../types";
import classes from "./ManageTracksModal.module.css";

const DAY_LABELS = ["M", "T", "W", "T", "F", "S", "S"];

const trackSchema = z.object({
	key: z.string(),
	label: z.string().trim().min(1, "Name is required").max(30, "Keep it under 30 characters"),
	defaultTime: z.string().regex(/^$|^([01]\d|2[0-3]):[0-5]\d$/, "Use HH:MM"),
	color: z.enum(TRACK_COLORS),
	visible: z.boolean(),
	repeatsOn: z.string().regex(/^[01]{7}$/),
});

const tracksSchema = z.object({
	tracks: z.array(trackSchema).min(1, "Keep at least one track"),
});

/** A track as the editor holds it. `defaultTime` is `""` for anytime. */
interface DraftTrack {
	key: string;
	label: string;
	defaultTime: string;
	color: string;
	visible: boolean;
	repeatsOn: string;
}

const toDraft = (track: Track): DraftTrack => ({
	key: track.key,
	label: track.label,
	defaultTime: track.defaultTime ?? "",
	color: track.color,
	visible: track.visible,
	repeatsOn: track.repeatsOn,
});

export function openManageTracksModal() {
	modals.openContextModal({ modal: "manageTracks", title: "Manage tracks", size: "xl", innerProps: {} });
}

export function ManageTracksModal({ context, id }: ContextModalProps) {
	const { data: tracks, isPending, isError, error } = useQuery(tracksQueryOptions);

	if (isError) return <Alert color="red" title="Could not load tracks">{String(error)}</Alert>;
	if (isPending) return <Loader size="md" m="auto" display="block" />;

	return <TracksEditor
		tracks={tracks}
		onCancel={() => context.closeModal(id)}
		onSaved={() => context.closeModal(id)}
	/>
}

interface TracksEditorProps {
	tracks: Track[];
	onCancel: () => void;
	onSaved: () => void;
}

function TracksEditor({ tracks, onCancel, onSaved }: TracksEditorProps) {
	const saveTracks = useSaveTracks();
	const [expandedKey, setExpandedKey] = useState<string | null>(null);
	const sensors = useSensors(useSensor(PointerSensor, { activationConstraint: { distance: 4 } }));

	const form = useForm({
		defaultValues: { tracks: tracks.map(toDraft) },
		validators: { onChange: tracksSchema },
		onSubmit: ({ value }) => {
			saveTracks.mutate(
				value.tracks.map((track) => ({
					key: track.key,
					label: track.label.trim(),
					color: track.color,
					defaultTime: track.defaultTime || null,
					visible: track.visible,
					repeatsOn: track.repeatsOn,
				})),
				{ onSuccess: onSaved },
			);
		},
	});

	return <form className={classes.editor} onSubmit={(event) => {
		event.preventDefault();
		form.handleSubmit();
	}}>
		<Stack gap="md">
			<Text size="sm" c="dimmed">Tracks are the rows of your weekly schedule. Reorder, recolor or hide them.</Text>

			<div className={classes.columnHeaders}>
				<div />
				<div />
				<Text size="xs" c="dimmed" className={classes.nameHeading}>Name</Text>
				<Text size="xs" c="dimmed">Default time</Text>
				<Text size="xs" c="dimmed" className={classes.showHeading}>Show</Text>
				<div />
			</div>

			<form.Field name="tracks" mode="array">
				{(tracksField) => <>
					<DndContext
						sensors={sensors}
						collisionDetection={closestCenter}
						onDragEnd={(event: DragEndEvent) => {
							if (!event.over || event.active.id === event.over.id) return;

							const current = tracksField.state.value.map((track) => track.key);
							tracksField.moveValue(current.indexOf(String(event.active.id)), current.indexOf(String(event.over.id)));
						}}
					>
						<SortableContext items={tracksField.state.value.map((track) => track.key)} strategy={verticalListSortingStrategy}>
							<Stack gap="xs">
								{tracksField.state.value.map((track, index) => (
									<SortableTrack
										key={track.key}
										id={track.key}
										expanded={expandedKey === track.key}
										onOpen={() => setExpandedKey(track.key)}
									>
										{(dragHandle) => <>
											<div className={classes.mainLine}>
												<ActionIcon
													ref={dragHandle.ref}
													{...dragHandle.attributes}
													{...dragHandle.listeners}
													variant="transparent"
													color="gray"
													size={28}
													aria-label={`Drag ${track.label} to reorder`}
													className={classes.handle}
												>
													<IconGripVertical size={16} stroke={1.5} />
												</ActionIcon>

												<ColorSwatch color={track.color} size={12} />

												<form.Field name={`tracks[${index}].label`}>
													{(field) => <TextInput
														flex={1}
														aria-label="Track name"
														value={field.state.value}
														onChange={(event) => field.handleChange(event.currentTarget.value)}
														onBlur={field.handleBlur}
														error={field.state.meta.isTouched && field.state.meta.errors.length > 0
															? field.state.meta.errors.map((issue) => issue?.message).join(", ")
															: undefined}
													/>}
												</form.Field>

												<form.Field name={`tracks[${index}].defaultTime`}>
													{(field) => <TextInput
														className={classes.time}
														aria-label="Default time"
														placeholder="Anytime"
														value={field.state.value}
														onChange={(event) => field.handleChange(event.currentTarget.value)}
														onBlur={field.handleBlur}
														error={field.state.meta.isTouched && field.state.meta.errors.length > 0
															? "HH:MM"
															: undefined}
													/>}
												</form.Field>

												<div className={classes.showCell}>
													<form.Field name={`tracks[${index}].visible`}>
														{(field) => <Switch
															aria-label={`Show ${track.label}`}
															checked={field.state.value}
															onChange={(event) => field.handleChange(event.currentTarget.checked)}
														/>}
													</form.Field>
												</div>

												<ActionIcon
													variant="subtle"
													color="gray"
													size={28}
													aria-label={`Remove ${track.label}`}
													onClick={() => tracksField.removeValue(index)}
												>
													<IconTrash size={16} stroke={1.5} />
												</ActionIcon>
											</div>

											{expandedKey === track.key && <div className={classes.details}>
												<div>
													<Text size="xs" c="dimmed" className={classes.sectionLabel}>Color</Text>
													<form.Field name={`tracks[${index}].color`}>
														{(field) => <Group gap="xs">
															{TRACK_COLORS.map((color) => (
																<ColorSwatch
																	key={color}
																	component="button"
																	type="button"
																	color={color}
																	size={24}
																	aria-label={`Color ${color}`}
																	aria-pressed={field.state.value === color}
																	onClick={() => field.handleChange(color)}
																	className={field.state.value === color ? classes.swatchSelected : classes.swatch}
																/>
															))}
														</Group>}
													</form.Field>
												</div>

												<div>
													<Text size="xs" c="dimmed" className={classes.sectionLabel}>Repeats on</Text>
													<form.Field name={`tracks[${index}].repeatsOn`}>
														{(field) => <Group gap={6} wrap="nowrap">
															{DAY_LABELS.map((label, day) => {
																const isOn = field.state.value[day] === "1";

																return <UnstyledButton
																	key={day}
																	aria-pressed={isOn}
																	aria-label={`Repeats on day ${day + 1}`}
																	className={isOn ? classes.dayOn : classes.day}
																	onClick={() => field.handleChange(
																		field.state.value.split("").map((char, i) => (i === day ? (isOn ? "0" : "1") : char)).join(""),
																	)}
																>
																	{label}
																</UnstyledButton>
															})}
														</Group>}
													</form.Field>
												</div>
											</div>}
										</>}
									</SortableTrack>
								))}
							</Stack>
						</SortableContext>
					</DndContext>

					{tracksField.state.meta.errors.length > 0 && <Text size="sm" c="red">
						{tracksField.state.meta.errors.map((issue) => issue?.message).join(", ")}
					</Text>}

					<UnstyledButton
						className={classes.addTrack}
						onClick={() => {
							const existing = tracksField.state.value;
							const key = makeTrackKey("New track", existing.map((track) => track.key));
							const color = TRACK_COLORS.find((c) => !existing.some((track) => track.color === c)) ?? TRACK_COLORS[existing.length % TRACK_COLORS.length];

							tracksField.pushValue({ key, label: "New track", defaultTime: "", color, visible: true, repeatsOn: ALL_DAYS });
							setExpandedKey(key);
						}}
					>
						<IconPlus size={14} stroke={1.5} />
						<Text size="sm">Add track</Text>
					</UnstyledButton>
				</>}
			</form.Field>

			{saveTracks.isError && <Alert color="red" title="Could not save the tracks">{String(saveTracks.error)}</Alert>}

			<Group justify="space-between" className={classes.footer}>
				<Text size="xs" c="dimmed">Hidden tracks keep their planned meals.</Text>
				<Group gap="sm">
					<Button variant="default" onClick={onCancel}>Cancel</Button>
					<Button type="submit" loading={saveTracks.isPending}>Save changes</Button>
				</Group>
			</Group>
		</Stack>
	</form>
}

interface SortableTrackProps {
	id: string;
	expanded: boolean;
	/** Opens the editing panel. Runs on click and on focus, so keyboard users get it too. */
	onOpen: () => void;
	children: (dragHandle: DragHandle) => ReactNode;
}

type Sortable = ReturnType<typeof useSortable>;

interface DragHandle {
	ref: Sortable["setActivatorNodeRef"];
	attributes: Sortable["attributes"];
	listeners: Sortable["listeners"];
}

/** One draggable track card. The drag handle is passed to `children` so only it starts a drag. */
function SortableTrack({ id, expanded, onOpen, children }: SortableTrackProps) {
	const { attributes, listeners, setNodeRef, setActivatorNodeRef, transform, transition, isDragging } = useSortable({ id });

	const className = [
		classes.track,
		expanded ? classes.trackExpanded : "",
	].join(" ");

	return <div
		ref={setNodeRef}
		style={{ transform: CSS.Transform.toString(transform), transition, opacity: isDragging ? 0.6 : undefined }}
		className={className}
		onClick={onOpen}
		onFocus={onOpen}
	>
		{children({ ref: setActivatorNodeRef, attributes, listeners })}
	</div>
}

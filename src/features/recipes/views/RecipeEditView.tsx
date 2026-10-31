import { useForm } from "@tanstack/react-form";
import { useSuspenseQuery } from "@tanstack/react-query";
import { Link, useNavigate } from "@tanstack/react-router";
import { IconChevronLeft, IconMinus, IconPlus } from "@tabler/icons-react";
import { modals } from "@mantine/modals";
import { ActionIcon, Alert, Button, Group, Image, Stack, TagsInput, Text, Textarea, TextInput, Title, UnstyledButton } from "@mantine/core";
import { z } from "zod";
import { tracksQueryOptions } from "../../planner/queries";
import { domainOf } from "../format";
import { useDeleteRecipe, useUpdateRecipe } from "../mutations";
import { recipeQueryOptions } from "../queries";
import { StarInput, StarRating } from "../components/StarRating";
import type { RecipeDetail, RecipeInput } from "../types";
import classes from "./RecipeEditView.module.css";

const MAX_SERVINGS = 99;

const isWebLink = (url: string) => /^https?:\/\/\S+$/.test(url);

/** One item per line in the text areas. Blank lines are dropped. */
const linesOf = (text: string) => text.split("\n").map((line) => line.trim()).filter(Boolean);

const recipeSchema = z.object({
	url: z.string().trim().refine((url) => url === "" || isWebLink(url), "Use a full link starting with http:// or https://"),
	imageUrl: z.string().trim().refine((url) => url === "" || isWebLink(url), "Use a full link starting with http:// or https://"),
	name: z.string().trim().min(1, "Give the recipe a title").max(120, "Keep it under 120 characters"),
	rating: z.number().int().min(1).max(5).nullable(),
	tags: z.array(z.string().trim().min(1)),
	usualTrackKey: z.string().nullable(),
	minutes: z.string().regex(/^$|^[1-9]\d{0,3}$/, "Use whole minutes"),
	servings: z.number().int().min(1).max(MAX_SERVINGS).nullable(),
	ingredients: z.string(),
	instructions: z.string(),
	notes: z.string(),
});

type RecipeFormValues = z.infer<typeof recipeSchema>;

const toFormValues = (recipe: RecipeDetail): RecipeFormValues => ({
	url: recipe.url,
	imageUrl: recipe.imageUrl ?? "",
	name: recipe.name,
	rating: recipe.rating ?? null,
	tags: recipe.tags,
	usualTrackKey: recipe.usualTrackKey,
	minutes: recipe.minutes ? String(recipe.minutes) : "",
	servings: recipe.servings ?? null,
	ingredients: recipe.ingredients.join("\n"),
	instructions: recipe.instructions.join("\n"),
	notes: recipe.notes ?? "",
});

const toInput = (values: RecipeFormValues): RecipeInput => ({
	name: values.name.trim(),
	url: values.url.trim(),
	sourceDomain: domainOf(values.url.trim()),
	minutes: values.minutes ? Number(values.minutes) : null,
	rating: values.rating,
	servings: values.servings,
	notes: values.notes.trim() || null,
	usualTrackKey: values.usualTrackKey,
	tags: values.tags,
	ingredients: linesOf(values.ingredients),
	instructions: linesOf(values.instructions),
	imageUrl: values.imageUrl.trim() || null,
});

interface RecipeEditViewProps {
	recipeId: string;
}

export function RecipeEditView({ recipeId }: RecipeEditViewProps) {
	const { data: recipe } = useSuspenseQuery(recipeQueryOptions(recipeId));
	const { data: tracks } = useSuspenseQuery(tracksQueryOptions);
	const navigate = useNavigate();
	const updateRecipe = useUpdateRecipe(recipe.id);
	const deleteRecipe = useDeleteRecipe();

	const goToDetail = () => navigate({ to: "/recipes/$recipeId", params: { recipeId: recipe.id } });

	const form = useForm({
		defaultValues: toFormValues(recipe),
		validators: { onChange: recipeSchema },
		onSubmit: ({ value }) => {
			updateRecipe.mutate(toInput(value), { onSuccess: goToDetail });
		},
	});

	const confirmDelete = () => modals.openConfirmModal({
		title: "Delete recipe?",
		children: <Text size="sm">This removes {recipe.name} from your library. Its planned meals are removed from the schedule too.</Text>,
		labels: { confirm: "Delete recipe", cancel: "Cancel" },
		confirmProps: { color: "red" },
		onConfirm: () => deleteRecipe.mutate(recipe.id, {
			onSuccess: () => navigate({ to: "/recipes" }),
		}),
	});

	return <form className={classes.view} onSubmit={(event) => {
		event.preventDefault();
		form.handleSubmit();
	}}>
		<header className={classes.header}>
			<div>
				<Link to="/recipes/$recipeId" params={{ recipeId: recipe.id }} className={classes.back}>
					<IconChevronLeft size={14} stroke={1.5} />
					{recipe.name}
				</Link>
				<Title order={1} mt="xs">Edit recipe</Title>
			</div>

			<form.Subscribe selector={(state) => state.isDirty}>
				{(isDirty) => isDirty && <Text size="sm" className={classes.dirty}>
					<span className={classes.dirtyDot} />
					Unsaved changes
				</Text>}
			</form.Subscribe>
		</header>

		<div className={classes.body}>
			<section className={classes.panel}>
				<Stack gap="lg">
					<form.Field name="url">
						{(field) => <TextInput
							label="Recipe link"
							placeholder="https://"
							value={field.state.value}
							onChange={(event) => field.handleChange(event.currentTarget.value)}
							onBlur={field.handleBlur}
							error={field.state.meta.isTouched && field.state.meta.errors.length > 0
								? field.state.meta.errors.map((issue) => issue?.message).join(", ")
								: undefined}
						/>}
					</form.Field>

					<form.Field name="imageUrl">
						{(field) => <TextInput
							label="Image link"
							placeholder="https://"
							value={field.state.value}
							onChange={(event) => field.handleChange(event.currentTarget.value)}
							onBlur={field.handleBlur}
							error={field.state.meta.isTouched && field.state.meta.errors.length > 0
								? field.state.meta.errors.map((issue) => issue?.message).join(", ")
								: undefined}
						/>}
					</form.Field>

					<form.Field name="name">
						{(field) => <TextInput
							label="Title"
							value={field.state.value}
							onChange={(event) => field.handleChange(event.currentTarget.value)}
							onBlur={field.handleBlur}
							error={field.state.meta.isTouched && field.state.meta.errors.length > 0
								? field.state.meta.errors.map((issue) => issue?.message).join(", ")
								: undefined}
						/>}
					</form.Field>

					<div>
						<Text size="sm" fw={500} mb={6}>Rating</Text>
						<form.Field name="rating">
							{(field) => <StarInput value={field.state.value} onChange={(rating) => field.handleChange(rating)} />}
						</form.Field>
					</div>

					<form.Field name="tags">
						{(field) => <TagsInput
							label="Tags"
							placeholder="Add a tag"
							value={field.state.value}
							onChange={(tags) => field.handleChange(tags)}
							onBlur={field.handleBlur}
						/>}
					</form.Field>

					<div>
						<Text size="sm" fw={500} mb={6}>Usually planned for</Text>
						<form.Field name="usualTrackKey">
							{(field) => <div className={classes.chips}>
								<UnstyledButton
									aria-pressed={field.state.value === null}
									onClick={() => field.handleChange(null)}
									className={field.state.value === null ? `${classes.chip} ${classes.chipActive}` : classes.chip}
								>
									None
								</UnstyledButton>
								{tracks.map((track) => (
									<UnstyledButton
										key={track.key}
										aria-pressed={field.state.value === track.key}
										onClick={() => field.handleChange(track.key)}
										className={field.state.value === track.key ? `${classes.chip} ${classes.chipActive}` : classes.chip}
									>
										{track.label}
									</UnstyledButton>
								))}
							</div>}
						</form.Field>
					</div>

					<Group align="flex-end" gap="lg" wrap="nowrap">
						<form.Field name="minutes">
							{(field) => <TextInput
								label="Cook time (min)"
								inputMode="numeric"
								className={classes.minutes}
								value={field.state.value}
								onChange={(event) => field.handleChange(event.currentTarget.value)}
								onBlur={field.handleBlur}
								error={field.state.meta.isTouched && field.state.meta.errors.length > 0
									? field.state.meta.errors.map((issue) => issue?.message).join(", ")
									: undefined}
							/>}
						</form.Field>

						<form.Field name="servings">
							{(field) => <div>
								<Text size="sm" fw={500} mb={6}>Serves</Text>
								<Group gap="xs" wrap="nowrap">
									<ActionIcon
										variant="default"
										size="lg"
										aria-label="Fewer servings"
										onClick={() => field.handleChange(Math.max(1, (field.state.value ?? 1) - 1))}
									>
										<IconMinus size={16} stroke={1.5} />
									</ActionIcon>
									<Text fw={700} className={classes.servings}>{field.state.value ?? "–"}</Text>
									<ActionIcon
										variant="default"
										size="lg"
										aria-label="More servings"
										onClick={() => field.handleChange(Math.min(MAX_SERVINGS, (field.state.value ?? 0) + 1))}
									>
										<IconPlus size={16} stroke={1.5} />
									</ActionIcon>
								</Group>
							</div>}
						</form.Field>
					</Group>

					<form.Field name="ingredients">
						{(field) => <Textarea
							label="Ingredients"
							description="One ingredient per line"
							autosize
							minRows={4}
							value={field.state.value}
							onChange={(event) => field.handleChange(event.currentTarget.value)}
							onBlur={field.handleBlur}
						/>}
					</form.Field>

					<form.Field name="instructions">
						{(field) => <Textarea
							label="Instructions"
							description="One step per line"
							autosize
							minRows={4}
							value={field.state.value}
							onChange={(event) => field.handleChange(event.currentTarget.value)}
							onBlur={field.handleBlur}
						/>}
					</form.Field>

					<form.Field name="notes">
						{(field) => <Textarea
							label="Notes"
							autosize
							minRows={4}
							value={field.state.value}
							onChange={(event) => field.handleChange(event.currentTarget.value)}
							onBlur={field.handleBlur}
						/>}
					</form.Field>

					{updateRecipe.isError && <Alert color="red" title="Could not save the recipe">{String(updateRecipe.error)}</Alert>}

					<Group justify="flex-end" className={classes.footer}>
						<Button variant="default" onClick={() => void goToDetail()}>Cancel</Button>
						<Button type="submit" loading={updateRecipe.isPending}>Save changes</Button>
					</Group>
				</Stack>
			</section>

			<div className={classes.column}>
				<section className={classes.panel}>
					<Text className={classes.panelTitle}>Preview in your list</Text>
					<form.Subscribe selector={(state) => state.values}>
						{(values) => <div className={classes.preview}>
							<div className={classes.avatar}>{(values.name || recipe.name).charAt(0)}</div>
							<div>
								<Text size="sm" fw={600}>{values.name || recipe.name}</Text>
								<Text size="xs" c="dimmed">{domainOf(values.url) || "No link yet"}</Text>
								<StarRating value={values.rating} />
							</div>
						</div>}
					</form.Subscribe>
				</section>

				<form.Subscribe selector={(state) => state.values.imageUrl}>
					{(imageUrl) => isWebLink(imageUrl) && <section className={classes.panel}>
						<Text className={classes.panelTitle}>Photo</Text>
						<div className={classes.photo}>
							<Image src={imageUrl} alt={recipe.name} />
						</div>
					</section>}
				</form.Subscribe>

				<section className={classes.panel}>
					<Text className={classes.panelTitle}>Delete recipe</Text>
					<Text size="sm" c="dimmed" mb="md">
						This removes the recipe from your library. Its planned meals are removed from the schedule too.
					</Text>
					<Button color="red" variant="outline" onClick={confirmDelete} loading={deleteRecipe.isPending}>
						Delete recipe
					</Button>
					{deleteRecipe.isError && <Alert color="red" title="Could not delete the recipe" mt="md">{String(deleteRecipe.error)}</Alert>}
				</section>
			</div>
		</div>
	</form>
}

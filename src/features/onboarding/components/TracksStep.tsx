import { IconPlus } from "@tabler/icons-react";
import { useSuspenseQuery } from "@tanstack/react-query";
import { ColorSwatch, Stack, Switch, Text, UnstyledButton } from "@mantine/core";
import { openManageTracksModal } from "../../planner/modals/ManageTracksModal";
import { tracksQueryOptions } from "../../planner/queries";
import { useSaveTracks } from "../../planner/mutations";
import { StepFooter } from "./StepFooter";
import { StepIntro } from "./StepIntro";
import classes from "./TracksStep.module.css";

interface TracksStepProps {
	onBack: () => void;
	onNext: () => void;
	onSkip: () => void;
}

export function TracksStep({ onBack, onNext, onSkip }: TracksStepProps) {
	const { data: tracks } = useSuspenseQuery(tracksQueryOptions);
	const saveTracks = useSaveTracks();

	// Each switch saves the whole list, the same way the tracks editor does.
	const setVisible = (key: string, visible: boolean) => saveTracks.mutate(
		tracks.map(({ position, ...track }) => track.key === key ? { ...track, visible } : track),
	);

	return <>
		<StepIntro
			step={2}
			title="Which meals do you plan?"
			description="Tracks are the rows of your schedule. Start with the usual three and tune them later."
		/>

		<Stack gap="xs">
			{tracks.map((track) => <div key={track.key} className={track.visible ? `${classes.row} ${classes.rowOn}` : classes.row}>
				<ColorSwatch color={track.color} size={12} withShadow={false} />
				<Text fw={600} className={classes.name}>{track.label}</Text>
				<Text size="sm" c="dimmed" className={classes.time}>{track.defaultTime ?? "Anytime"}</Text>
				<Switch
					checked={track.visible}
					onChange={(event) => setVisible(track.key, event.currentTarget.checked)}
					disabled={saveTracks.isPending}
					aria-label={`Plan ${track.label}`}
				/>
			</div>)}

			<UnstyledButton className={classes.addTrack} onClick={() => openManageTracksModal()}>
				<IconPlus size={16} stroke={1.5} />
				<span>Add your own track</span>
			</UnstyledButton>
		</Stack>

		<Text size="sm" c="dimmed">You can reorder, recolor or hide tracks later from the Tracks button.</Text>

		<StepFooter onBack={onBack} onSkip={onSkip} onNext={onNext} />
	</>
}

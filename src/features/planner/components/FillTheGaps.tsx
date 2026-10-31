import { IconPlus } from "@tabler/icons-react";
import { ActionIcon, Text } from "@mantine/core";
import { scheduleTracks } from "../config/tracks";
import { mockGapSuggestions } from "../data/mockPlan";
import classes from "./FillTheGaps.module.css";

export function FillTheGaps() {
	return <div className={classes.panel}>
		<Text size="lg" fw={600} mb="md">Fill the gaps</Text>

		<div className={classes.list}>
			{mockGapSuggestions.map((suggestion) => {
				const trackLabel = scheduleTracks.find((track) => track.key === suggestion.trackKey)?.label;

				return <div key={suggestion.recipe.id} className={classes.item}>
					<div>
						<Text size="sm" fw={500}>{suggestion.recipe.name}</Text>
						<Text size="xs" c="dimmed">{trackLabel} · {suggestion.reason}</Text>
					</div>
					<ActionIcon variant="filled" aria-label={`Add ${suggestion.recipe.name}`}>
						<IconPlus size={14} stroke={1.5} />
					</ActionIcon>
				</div>
			})}
		</div>
	</div>
}

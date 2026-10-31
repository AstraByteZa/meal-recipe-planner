import { Fragment } from "react";
import { IconPlus } from "@tabler/icons-react";
import { useSuspenseQuery } from "@tanstack/react-query";
import { Menu, Text, UnstyledButton } from "@mantine/core";
import dayjs from "../../../lib/dayjs";
import { useSetScheduleEntry } from "../../planner/mutations";
import { scheduleQueryOptions, tracksQueryOptions } from "../../planner/queries";
import { DATE_FORMAT, currentWeek } from "../../planner/week";
import { repeatsOnDay } from "../../planner/trackSettings";
import { recipesQueryOptions } from "../../recipes/queries";
import { StepFooter } from "./StepFooter";
import { StepIntro } from "./StepIntro";
import classes from "./FirstPlanStep.module.css";

const DAY_LABELS = ["M", "T", "W", "T", "F", "S", "S"];

interface FirstPlanStepProps {
	onBack: () => void;
	onFinish: () => void;
	finishing: boolean;
}

/** Shows this week's visible tracks. Tapping a slot puts one of the saved recipes in it. */
export function FirstPlanStep({ onBack, onFinish, finishing }: FirstPlanStepProps) {
	const week = currentWeek();
	const { data: tracks } = useSuspenseQuery(tracksQueryOptions);
	const { data: recipes } = useSuspenseQuery(recipesQueryOptions);
	const { data: plan } = useSuspenseQuery(scheduleQueryOptions(week));
	const setEntry = useSetScheduleEntry();

	const visibleTracks = tracks.filter((track) => track.visible);
	const days = DAY_LABELS.map((_, index) => dayjs(week).add(index, "day"));

	const plannedCount = days.reduce((count, day) => {
		const date = day.format(DATE_FORMAT);
		return count + visibleTracks.filter((track) => plan[date]?.[track.key]).length;
	}, 0);

	return <>
		<StepIntro
			step={5}
			title="Drop a recipe into your week"
			description="Your first plan is one tap away."
		/>

		<div className={classes.board}>
			<div className={classes.grid}>
				<div />
				{DAY_LABELS.map((label, index) => <Text key={index} size="xs" c="dimmed" ta="center">{label}</Text>)}

				{visibleTracks.map((track) => <Fragment key={track.key}>
					<Text size="xs" fw={700} c={track.color} className={classes.trackLabel}>{track.label.toUpperCase()}</Text>

					{days.map((day) => {
						const date = day.format(DATE_FORMAT);
						if (!repeatsOnDay(track, day)) return <div key={date} className={classes.slotOff} />;

						const meal = plan[date]?.[track.key];

						return <Menu key={date} position="bottom" withinPortal>
							<Menu.Target>
								<UnstyledButton
									className={meal ? classes.slotFilled : classes.slotEmpty}
									aria-label={meal ? `Change ${track.label} on ${date}` : `Add to ${track.label} on ${date}`}
								>
									{meal ? <span className={classes.mealName}>{meal.name}</span> : <IconPlus size={16} stroke={1.5} />}
								</UnstyledButton>
							</Menu.Target>

							<Menu.Dropdown>
								{recipes.length === 0
									? <Menu.Label>Save a link first</Menu.Label>
									: recipes.map((recipe) => <Menu.Item
										key={recipe.id}
										onClick={() => setEntry.mutate({ date, trackKey: track.key, recipeId: recipe.id })}
									>
										{recipe.name}
									</Menu.Item>)}
							</Menu.Dropdown>
						</Menu>
					})}
				</Fragment>)}
			</div>
		</div>

		<Text size="sm" c="dimmed">
			{plannedCount > 0
				? `${plannedCount} planned this week. Tap any slot to add more or swap a meal.`
				: "Tap a slot to put one of your saved links there."}
		</Text>

		<StepFooter onBack={onBack} onNext={onFinish} nextLabel="Open my schedule" nextLoading={finishing} />
	</>
}

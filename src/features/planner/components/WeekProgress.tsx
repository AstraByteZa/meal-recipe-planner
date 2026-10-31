import { useQuery } from "@tanstack/react-query";
import { Progress, Text } from "@mantine/core";
import dayjs from "../../../lib/dayjs";
import { scheduleQueryOptions, tracksQueryOptions } from "../queries";
import { repeatsOnDay } from "../trackSettings";
import { DATE_FORMAT, currentWeek } from "../week";
import classes from "./WeekProgress.module.css";

/** Planned share of this week's open schedule cells. Matches what the planner grid shows. */
export function WeekProgress() {
	const week = currentWeek();
	const tracks = useQuery(tracksQueryOptions);
	const schedule = useQuery(scheduleQueryOptions(week));

	if (tracks.isError || schedule.isError) {
		return <div className={classes.card}>
			<Text size="sm" fw={600}>This week</Text>
			<Text size="xs" c="dimmed">Could not load the week</Text>
		</div>
	}

	if (!tracks.data || !schedule.data) return null;

	const weekStart = dayjs(week);
	let totalSlots = 0;
	let plannedMeals = 0;

	for (let i = 0; i < 7; i++) {
		const day = weekStart.add(i, "day");
		const dayPlan = schedule.data[day.format(DATE_FORMAT)];

		for (const track of tracks.data) {
			if (!track.visible) continue;

			const meal = dayPlan?.[track.key];
			// A cell is open when the track repeats that day, or when a meal is already there.
			if (!meal && !repeatsOnDay(track, day)) continue;

			totalSlots++;
			if (meal) plannedMeals++;
		}
	}

	const percent = totalSlots === 0 ? 0 : (plannedMeals / totalSlots) * 100;

	return <div className={classes.card}>
		<Text size="sm" fw={600}>This week</Text>
		<Text size="xs" c="dimmed" mb="xs">{plannedMeals} of {totalSlots} meals planned</Text>
		<Progress value={percent} size="sm" radius="xl" />
	</div>
}

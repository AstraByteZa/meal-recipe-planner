import { queryOptions } from "@tanstack/react-query";
import { invoke } from "@tauri-apps/api/core";
import dayjs from "../../lib/dayjs";
import type { PlanByDate, PlannedMeal, Track } from "./types";
import { DATE_FORMAT, PAST_WEEK_COUNT, shiftWeek } from "./week";

export const tracksQueryOptions = queryOptions({
	queryKey: ["tracks"],
	queryFn: () => invoke<Track[]>("tracks_list"),
});

/** Planned meals for the shown week and the past weeks listed beside it. */
export function scheduleQueryOptions(week: string) {
	const start = shiftWeek(week, -PAST_WEEK_COUNT);
	const end = dayjs(week).add(6, "day").format(DATE_FORMAT);

	return queryOptions({
		queryKey: ["schedule", start, end],
		queryFn: async () => groupByDate(await invoke<PlannedMeal[]>("schedule_week", { start, end })),
	});
}

function groupByDate(meals: PlannedMeal[]): PlanByDate {
	const plan: PlanByDate = {};

	for (const meal of meals) {
		plan[meal.date] = { ...plan[meal.date], [meal.trackKey]: meal };
	}

	return plan;
}

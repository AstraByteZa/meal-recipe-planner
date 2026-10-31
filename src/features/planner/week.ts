import dayjs from "../../lib/dayjs";

/** Format of the `week` search param and of schedule dates. */
export const DATE_FORMAT = "YYYY-MM-DD";

/** Past weeks listed beside the planner. */
export const PAST_WEEK_COUNT = 5;

/** Monday of the current ISO week, as `YYYY-MM-DD`. */
export const currentWeek = () => dayjs().startOf("isoWeek").format(DATE_FORMAT);

/** Moves a week by a number of weeks. */
export const shiftWeek = (week: string, weeks: number) =>
  dayjs(week).add(weeks, "week").format(DATE_FORMAT);

/** Snaps a search value to the Monday of its ISO week. Falls back to the current week when it is not a date. */
export function toWeekStart(value: unknown): string {
  if (typeof value !== "string" || !/^\d{4}-\d{2}-\d{2}$/.test(value) || !dayjs(value).isValid()) {
    return currentWeek();
  }

  return dayjs(value).startOf("isoWeek").format(DATE_FORMAT);
}

import dayjs from "../../lib/dayjs";

/** "26 Oct – 1 Nov", or "19 – 25 Oct" when both ends share a month. */
export function formatWeekRange(weekStart: dayjs.Dayjs) {
  const weekEnd = weekStart.add(6, "day");
  const startLabel =
    weekStart.month() === weekEnd.month()
      ? weekStart.format("D")
      : weekStart.format("D MMM");

  return `${startLabel} – ${weekEnd.format("D MMM")}`;
}

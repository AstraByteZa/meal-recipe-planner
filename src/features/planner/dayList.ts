import dayjs from "../../lib/dayjs";
import { DATE_FORMAT } from "./week";

/**
 * The day the narrow-screen list shows. The user's pick wins while it is in the week. Otherwise
 * today, if the week has it, and then the first day of the week.
 */
export function pickDay(days: dayjs.Dayjs[], picked: string | null, today: dayjs.Dayjs): dayjs.Dayjs {
	const pickedDay = picked ? days.find((day) => day.format(DATE_FORMAT) === picked) : undefined;
	return pickedDay ?? days.find((day) => day.isSame(today, "day")) ?? days[0];
}

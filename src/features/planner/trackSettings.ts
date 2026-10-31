import type dayjs from "dayjs";
import type { Track } from "./types";

/** Palette for track colors. These are Mantine theme color names. */
export const TRACK_COLORS = ["yellow", "myColor", "blue", "pink", "violet", "teal"] as const;

/** Every day of the week, Monday first. */
export const ALL_DAYS = "1111111";

/** Whether a track applies on the given day. */
export function repeatsOnDay(track: Track, day: dayjs.Dayjs) {
	// isoWeekday is 1 (Monday) to 7 (Sunday).
	return track.repeatsOn[day.isoWeekday() - 1] === "1";
}

/** A stable track key from a name. Adds a suffix when the key is taken. */
export function makeTrackKey(label: string, taken: string[]) {
	const base = label.toLowerCase().trim().replace(/[^a-z0-9]+/g, "-").replace(/^-|-$/g, "") || "track";

	let key = base;
	for (let n = 2; taken.includes(key); n++) {
		key = `${base}-${n}`;
	}
	return key;
}

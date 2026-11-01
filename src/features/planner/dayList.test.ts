import { describe, expect, it } from "vitest";
import dayjs from "../../lib/dayjs";
import { pickDay } from "./dayList";

const weekStart = dayjs("2026-10-26");
const days = Array.from({ length: 7 }, (_, i) => weekStart.add(i, "day"));
const today = dayjs("2026-10-28");

describe("pickDay", () => {
	it("keeps the user's pick when it is in the week", () => {
		expect(pickDay(days, "2026-10-30", today).format("YYYY-MM-DD")).toBe("2026-10-30");
	});

	it("falls back to today when the pick is outside the week", () => {
		expect(pickDay(days, "2026-11-05", today).format("YYYY-MM-DD")).toBe("2026-10-28");
	});

	it("shows today when nothing is picked", () => {
		expect(pickDay(days, null, today).format("YYYY-MM-DD")).toBe("2026-10-28");
	});

	it("shows the first day when today is not in the week", () => {
		expect(pickDay(days, null, dayjs("2026-11-10")).format("YYYY-MM-DD")).toBe("2026-10-26");
	});
});

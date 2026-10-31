import { describe, expect, it } from "vitest";
import {
	addMinute,
	finishTimer,
	formatRemaining,
	isDue,
	MINUTE_MS,
	newTimer,
	pauseTimer,
	remainingAt,
	resetTimer,
	setTimerMinutes,
	startTimer,
	timersForStep,
} from "./timers";

const roast = newTimer({ id: "3-0", label: "Timer", step: 4, minutes: 10 });
const T0 = 1_000_000;

describe("timersForStep", () => {
	it("makes one timer per detected duration", () => {
		expect(timersForStep(2, "Cook 10 minutes, then rest 5 minutes")).toEqual([
			{ id: "2-0", label: "Timer 1 of 2", step: 3, minutes: 10 },
			{ id: "2-1", label: "Timer 2 of 2", step: 3, minutes: 5 },
		]);
	});

	it("offers a manual timer when the step has no duration", () => {
		expect(timersForStep(0, "Chop the onion")).toEqual([
			{ id: "0-manual", label: "Timer", step: 1, minutes: 5 },
		]);
	});
});

describe("timer state", () => {
	it("starts from the time left and counts down to endsAt", () => {
		const running = startTimer(roast, T0);
		expect(running.status).toBe("running");
		expect(remainingAt(running, T0 + 4_000)).toBe(10 * MINUTE_MS - 4_000);
	});

	it("keeps the time left when paused and resumes from it", () => {
		const paused = pauseTimer(startTimer(roast, T0), T0 + 60_000);
		expect(paused.status).toBe("paused");
		expect(remainingAt(paused, T0 + 999_999)).toBe(9 * MINUTE_MS);
		const resumed = startTimer(paused, T0 + 120_000);
		expect(remainingAt(resumed, T0 + 120_000)).toBe(9 * MINUTE_MS);
	});

	it("adds a minute to a running timer by moving endsAt", () => {
		const extended = addMinute(startTimer(roast, T0), T0);
		expect(remainingAt(extended, T0)).toBe(11 * MINUTE_MS);
	});

	it("adds a minute to an idle timer by pausing it with the extra time", () => {
		const extended = addMinute(roast, T0);
		expect(extended.status).toBe("paused");
		expect(extended.remainingMs).toBe(11 * MINUTE_MS);
	});

	it("is due only once a running timer reaches endsAt", () => {
		const running = startTimer(roast, T0);
		expect(isDue(running, T0 + 10 * MINUTE_MS - 1)).toBe(false);
		expect(isDue(running, T0 + 10 * MINUTE_MS)).toBe(true);
		expect(isDue(pauseTimer(running, T0), T0 + 10 * MINUTE_MS)).toBe(false);
	});

	it("finishes at zero and starts again from the full time", () => {
		const done = finishTimer(startTimer(roast, T0));
		expect(done.status).toBe("done");
		expect(remainingAt(done, T0 + 1)).toBe(0);
		const restarted = startTimer(done, T0 + 1);
		expect(remainingAt(restarted, T0 + 1)).toBe(10 * MINUTE_MS);
	});

	it("sets the length of an idle timer, and reset keeps the new length", () => {
		const longer = setTimerMinutes(roast, 55);
		expect(longer.totalMs).toBe(55 * MINUTE_MS);
		expect(longer.remainingMs).toBe(55 * MINUTE_MS);
		expect(resetTimer(startTimer(longer, T0)).remainingMs).toBe(55 * MINUTE_MS);
	});

	it("keeps the length of a running or paused timer", () => {
		const running = startTimer(roast, T0);
		expect(setTimerMinutes(running, 55)).toBe(running);
		const paused = pauseTimer(running, T0 + 60_000);
		expect(setTimerMinutes(paused, 55)).toBe(paused);
	});

	it("keeps a set length within 1 to 999 minutes", () => {
		expect(setTimerMinutes(roast, 0).totalMs).toBe(MINUTE_MS);
		expect(setTimerMinutes(roast, 5000).totalMs).toBe(999 * MINUTE_MS);
		expect(setTimerMinutes(roast, 54.6).totalMs).toBe(55 * MINUTE_MS);
	});

	it("reset returns an idle timer with its full time", () => {
		const reset = resetTimer(pauseTimer(startTimer(roast, T0), T0 + 30_000));
		expect(reset.status).toBe("idle");
		expect(reset.remainingMs).toBe(10 * MINUTE_MS);
	});
});

describe("formatRemaining", () => {
	it("shows minutes and seconds", () => {
		expect(formatRemaining(18 * 60_000 + 42_000)).toBe("18:42");
		expect(formatRemaining(0)).toBe("00:00");
	});

	it("rounds partial seconds up", () => {
		expect(formatRemaining(1)).toBe("00:01");
		expect(formatRemaining(59_001)).toBe("01:00");
	});

	it("adds hours past one hour", () => {
		expect(formatRemaining(75 * 60_000)).toBe("1:15:00");
	});
});

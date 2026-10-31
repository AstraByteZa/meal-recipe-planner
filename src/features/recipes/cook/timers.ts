import { durationsIn } from "./steps";

export const MINUTE_MS = 60_000;
/** Length of the manual timer offered for steps with no detected duration. */
const MANUAL_MINUTES = 5;

export type TimerStatus = "idle" | "running" | "paused" | "done";

/** A timer the current step offers, before the user has touched it. */
export interface TimerDef {
	id: string;
	label: string;
	/** One-based number of the step the timer belongs to. */
	step: number;
	minutes: number;
}

/**
 * A timer the user has interacted with. While running, `endsAt` is the timestamp it finishes at,
 * so the countdown stays right when the window is in the background. While idle, paused or done,
 * `remainingMs` holds the time left.
 */
export interface CookTimer extends TimerDef {
	totalMs: number;
	status: TimerStatus;
	remainingMs: number;
	endsAt: number | null;
}

/** The timers for a step: one per detected duration, or a manual timer when the step has none. */
export function timersForStep(index: number, text: string): TimerDef[] {
	const step = index + 1;
	const durations = durationsIn(text);
	if (durations.length === 0) {
		return [{ id: `${index}-manual`, label: "Timer", step, minutes: MANUAL_MINUTES }];
	}
	return durations.map((minutes, i) => ({
		id: `${index}-${i}`,
		label: durations.length > 1 ? `Timer ${i + 1} of ${durations.length}` : "Timer",
		step,
		minutes,
	}));
}

export function newTimer(def: TimerDef): CookTimer {
	const totalMs = def.minutes * MINUTE_MS;
	return { ...def, totalMs, status: "idle", remainingMs: totalMs, endsAt: null };
}

/** Longest timer the user can set by hand, in minutes. */
export const MAX_MINUTES = 999;

/** Sets a new length for an idle timer. A running or paused timer keeps its time, so the user resets it first. */
export function setTimerMinutes(timer: CookTimer, minutes: number): CookTimer {
	if (timer.status !== "idle") return timer;
	const totalMs = Math.min(MAX_MINUTES, Math.max(1, Math.round(minutes))) * MINUTE_MS;
	return { ...timer, totalMs, remainingMs: totalMs };
}

export function remainingAt(timer: CookTimer, now: number): number {
	return timer.status === "running" && timer.endsAt !== null ? Math.max(0, timer.endsAt - now) : timer.remainingMs;
}

export function startTimer(timer: CookTimer, now: number): CookTimer {
	if (timer.status === "running") return timer;
	const base = timer.status === "done" ? resetTimer(timer) : timer;
	return { ...base, status: "running", endsAt: now + base.remainingMs };
}

export function pauseTimer(timer: CookTimer, now: number): CookTimer {
	if (timer.status !== "running") return timer;
	return { ...timer, status: "paused", remainingMs: remainingAt(timer, now), endsAt: null };
}

/** Adds a minute to the time left, whatever the timer's state. */
export function addMinute(timer: CookTimer, now: number): CookTimer {
	if (timer.status === "running" && timer.endsAt !== null) {
		return { ...timer, endsAt: timer.endsAt + MINUTE_MS };
	}
	return { ...timer, status: "paused", remainingMs: remainingAt(timer, now) + MINUTE_MS, endsAt: null };
}

export function resetTimer(timer: CookTimer): CookTimer {
	return { ...timer, status: "idle", remainingMs: timer.totalMs, endsAt: null };
}

/** True once a running timer has reached its end. */
export function isDue(timer: CookTimer, now: number): boolean {
	return timer.status === "running" && timer.endsAt !== null && timer.endsAt <= now;
}

export function finishTimer(timer: CookTimer): CookTimer {
	return { ...timer, status: "done", remainingMs: 0, endsAt: null };
}

/** Shows whole seconds, rounded up so a timer never reads 00:00 while it still has time. Adds hours past one hour. */
export function formatRemaining(ms: number): string {
	const total = Math.ceil(ms / 1000);
	const hours = Math.floor(total / 3600);
	const minutes = Math.floor((total % 3600) / 60);
	const seconds = total % 60;
	const pad = (value: number) => String(value).padStart(2, "0");
	const clock = `${pad(minutes)}:${pad(seconds)}`;
	return hours > 0 ? `${hours}:${clock}` : clock;
}

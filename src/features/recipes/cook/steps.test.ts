import { describe, expect, it } from "vitest";
import { durationsIn, timeLeft, toCookStep } from "./steps";

describe("durationsIn", () => {
	it("reads a single minute or hour amount", () => {
		expect(durationsIn("Simmer for 20 minutes.")).toEqual([20]);
		expect(durationsIn("Rest 5 min")).toEqual([5]);
		expect(durationsIn("Roast 2 hours")).toEqual([120]);
		expect(durationsIn("Chill 1 hr")).toEqual([60]);
	});

	it("combines an hour with the minutes that follow it", () => {
		expect(durationsIn("Simmer 1 hour 15 min")).toEqual([75]);
		expect(durationsIn("Braise for 1 hour and 15 minutes")).toEqual([75]);
	});

	it("accepts decimal amounts", () => {
		expect(durationsIn("Proof 1.5 hours")).toEqual([90]);
	});

	it("uses the upper bound of a range", () => {
		expect(durationsIn("Bake 10–12 minutes")).toEqual([12]);
		expect(durationsIn("Bake 10-12 min")).toEqual([12]);
		expect(durationsIn("Bake 10 to 12 minutes")).toEqual([12]);
	});

	it("returns every separate duration in a step", () => {
		expect(durationsIn("Cook 10 minutes, then rest 5 minutes")).toEqual([10, 5]);
		expect(durationsIn("Simmer 12 minutes more")).toEqual([12]);
	});

	it("does not join separate minute amounts", () => {
		expect(durationsIn("Stir 10 minutes and 5 minutes")).toEqual([10, 5]);
	});

	it("returns nothing when there is no timed amount", () => {
		expect(durationsIn("Chop the onion finely.")).toEqual([]);
		expect(durationsIn("Serve 2 bowls")).toEqual([]);
		expect(durationsIn("Hourly check of the stock")).toEqual([]);
		expect(durationsIn("Rest 0 minutes")).toEqual([]);
	});
});

describe("timeLeft", () => {
	const steps = ["Chop the onion", "Sweat 5 minutes", "Simmer 20 minutes", "Serve"];

	it("sums the durations from the current step to the end", () => {
		expect(timeLeft(steps, 0)).toBe(25);
		expect(timeLeft(steps, 1)).toBe(25);
		expect(timeLeft(steps, 2)).toBe(20);
	});

	it("is zero when no timed step remains", () => {
		expect(timeLeft(steps, 3)).toBe(0);
		expect(timeLeft(steps, steps.length)).toBe(0);
		expect(timeLeft([], 0)).toBe(0);
	});
});

describe("toCookStep", () => {
	it("reads a whole number from 0 up", () => {
		expect(toCookStep("0")).toBe(0);
		expect(toCookStep("2")).toBe(2);
		expect(toCookStep(4)).toBe(4);
	});

	it("reads done as the screen after the last step", () => {
		expect(toCookStep("done")).toBe("done");
	});

	it("falls back to the first step for anything else", () => {
		expect(toCookStep(undefined)).toBe(0);
		expect(toCookStep("finish")).toBe(0);
		expect(toCookStep(-1)).toBe(0);
		expect(toCookStep(1.5)).toBe(0);
	});
});

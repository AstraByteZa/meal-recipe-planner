// A duration is a number, optionally a range ("10–12 minutes"), then an hour or minute unit.
// The second number of a range is captured too, so the upper bound can be used.
const NUMBER = String.raw`(\d+(?:\.\d+)?)`;
const AMOUNT = new RegExp(
	String.raw`${NUMBER}(?:\s*(?:-|–|—|to)\s*${NUMBER})?\s*(hours?|hrs?|minutes?|mins?)\b`,
	"gi",
);
// Text between "1 hour" and "15 min" that still makes them one duration: "1 hour 15 min", "1 hour and 15 minutes".
const COMPOUND_GAP = /^\s+(and\s+)?$/i;

type Amount = { minutes: number; hours: boolean; start: number; end: number };

function amountsIn(text: string): Amount[] {
	return [...text.matchAll(AMOUNT)].map((match) => {
		const hours = match[3].toLowerCase().startsWith("h");
		const value = Number(match[2] ?? match[1]);
		return {
			minutes: hours ? value * 60 : value,
			hours,
			start: match.index,
			end: match.index + match[0].length,
		};
	});
}

/** Every timer length in the text, in whole minutes. Ranges use the upper bound; zero-length amounts are ignored. */
export function durationsIn(text: string): number[] {
	const amounts = amountsIn(text);
	const durations: number[] = [];
	for (let i = 0; i < amounts.length; i++) {
		const amount = amounts[i];
		const next = amounts[i + 1];
		let minutes = amount.minutes;
		if (amount.hours && next && !next.hours && COMPOUND_GAP.test(text.slice(amount.end, next.start))) {
			minutes += next.minutes;
			i++;
		}
		durations.push(Math.round(minutes));
	}
	return durations.filter((minutes) => minutes > 0);
}

/**
 * Reads the `step` search param: a zero-based step index, or "done" for the screen after the last step.
 * Anything else falls back to the first step.
 */
export function toCookStep(value: unknown): number | "done" {
	if (value === "done") return "done";
	const step = Number(value);
	return Number.isInteger(step) && step >= 0 ? step : 0;
}

/** Minutes still to cook from the current step to the end: the sum of every detected duration from that step on. */
export function timeLeft(steps: string[], currentIndex: number): number {
	return steps
		.slice(currentIndex)
		.reduce((total, step) => total + durationsIn(step).reduce((sum, minutes) => sum + minutes, 0), 0);
}

export const STEPS = [
	{ title: "Welcome", subtitle: "What Mise does" },
	{ title: "Tracks", subtitle: "Choose your meals" },
	{ title: "Household", subtitle: "Who you cook for" },
	{ title: "First recipe", subtitle: "Save a link" },
	{ title: "First plan", subtitle: "Fill a slot" },
];

export const STEP_COUNT = STEPS.length;

/** Reads the `step` search param. Anything that is not a step number falls back to the first step. */
export function toStep(value: unknown): number {
	const step = Number(value);
	return Number.isInteger(step) && step >= 1 && step <= STEP_COUNT ? step : 1;
}

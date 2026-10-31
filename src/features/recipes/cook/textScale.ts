import type { AppConfig } from "../../config/schema";

export type TextScale = AppConfig["cook-text-scale"];

/** The sizes A- and A+ step through, smallest first. */
export const TEXT_SCALES: TextScale[] = ["sm", "md", "lg"];

/**
 * Multiplies the cook page's text sizes. The page sets it as `--cook-text-scale`, and its CSS modules
 * scale their font sizes by it. Fixed steps, so each A- or A+ press moves one size.
 */
export const TEXT_SCALE_FACTORS: Record<TextScale, number> = {
	sm: 0.875,
	md: 1,
	lg: 1.25,
};

/** Household settings and onboarding state. Matches `Settings` from the `settings_get` command. */
export interface Settings {
	householdSize: number;
	weekStartsOn: WeekStart;
	onboarded: boolean;
	/** Keys from `DIETARY_NEEDS`. Empty when the household has no dietary needs. */
	dietary: string[];
}

/** Payload for `settings_save`. */
export interface SettingsInput {
	householdSize: number;
	weekStartsOn: WeekStart;
	dietary: string[];
}

export type WeekStart = "monday" | "sunday";

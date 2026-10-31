export interface Recipe {
  id: string;
  name: string;
  minutes: number;
  rating?: number;
}

/** A schedule row. Matches `Track` from the `tracks_list` command. */
export interface Track {
  key: string;
  label: string;
  /** Mantine color name for the row label. */
  color: string;
  position: number;
  /** Start time as `HH:MM`, or null for "anytime". */
  defaultTime: string | null;
  /** Hidden tracks keep their planned meals but are not shown in the grid. */
  visible: boolean;
  /** Seven characters, Monday first. `1` means the track applies that day. */
  repeatsOn: string;
}

/** A recipe planned in one slot. Matches `PlannedMeal` from the `schedule_week` command. */
export interface PlannedMeal {
  date: string;
  trackKey: string;
  recipeId: string;
  name: string;
  minutes: number | null;
  rating: number | null;
}

/** Planned meal per track key for a single day. A missing key means nothing is planned. */
export type DayPlan = Partial<Record<string, PlannedMeal>>;

/** Plan keyed by ISO date (`YYYY-MM-DD`). */
export type PlanByDate = Record<string, DayPlan>;

export interface GapSuggestion {
  recipe: Recipe;
  trackKey: string;
  reason: string;
}

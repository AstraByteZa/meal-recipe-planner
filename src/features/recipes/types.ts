export interface SavedRecipe {
	id: string;
	name: string;
	sourceDomain: string;
	tags: string[];
	rating?: number | null;
	/** Days since the recipe was last planned, or null if it has never been planned. */
	lastUsedDaysAgo: number | null;
	/** Days since the link was saved. */
	savedDaysAgo: number;
	plannedCount: number;
}

/** One planned use of a recipe. Matches `PlanEntry` from the backend. */
export interface PlanEntry {
	/** ISO date, `YYYY-MM-DD`. */
	date: string;
	trackKey: string;
	trackLabel: string;
}

/** A recipe with everything the detail and edit views show. Matches `RecipeDetail` from the backend. */
export interface RecipeDetail extends SavedRecipe {
	url: string;
	minutes: number | null;
	servings: number | null;
	notes: string | null;
	imageUrl: string | null;
	ingredients: string[];
	instructions: string[];
	/** Track the recipe is usually planned on, or null if not set. */
	usualTrackKey: string | null;
	/** Newest first. */
	history: PlanEntry[];
}

/** A recipe read from a web page. Matches `ScrapedRecipe` from the backend. */
export interface ScrapedRecipe {
	name: string;
	sourceUrl: string;
	imageUrl: string | null;
	ingredients: string[];
	instructions: string[];
}

/** Payload for creating or replacing a recipe. Matches `RecipeInput` from the backend. */
export interface RecipeInput {
	name: string;
	url: string;
	sourceDomain: string;
	minutes: number | null;
	rating: number | null;
	servings: number | null;
	notes: string | null;
	usualTrackKey: string | null;
	tags: string[];
	ingredients: string[];
	instructions: string[];
	imageUrl: string | null;
}

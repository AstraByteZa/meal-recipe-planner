import type { RecipeDetail, RecipeInput } from "./types";

/**
 * The save payload for a stored recipe, with every field carried over. Start from this and change the
 * fields you edit, so a save never drops a field the page does not show.
 */
export function recipeToInput(recipe: RecipeDetail): RecipeInput {
	return {
		name: recipe.name,
		url: recipe.url,
		sourceDomain: recipe.sourceDomain,
		minutes: recipe.minutes,
		rating: recipe.rating ?? null,
		servings: recipe.servings,
		notes: recipe.notes,
		usualTrackKey: recipe.usualTrackKey,
		tags: recipe.tags,
		ingredients: recipe.ingredients,
		instructions: recipe.instructions,
		imageUrl: recipe.imageUrl,
	};
}

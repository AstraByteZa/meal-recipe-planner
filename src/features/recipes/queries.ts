import { queryOptions } from "@tanstack/react-query";
import { invoke } from "@tauri-apps/api/core";
import type { RecipeDetail, SavedRecipe } from "./types";

export const recipesQueryOptions = queryOptions({
	queryKey: ["recipes"],
	queryFn: () => invoke<SavedRecipe[]>("recipes_list"),
});

export function recipeQueryOptions(id: string) {
	return queryOptions({
		queryKey: ["recipes", id],
		queryFn: () => invoke<RecipeDetail>("recipe_get", { id }),
	});
}

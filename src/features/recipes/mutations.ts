import { useMutation, useQueryClient } from "@tanstack/react-query";
import { invoke } from "@tauri-apps/api/core";
import { domainOf, titleFromLink } from "./format";
import type { RecipeDetail, RecipeInput, ScrapedRecipe } from "./types";

/** Every field empty. The create hooks override the fields they set. */
const blankRecipe: RecipeInput = {
	name: "",
	url: "",
	sourceDomain: "",
	minutes: null,
	rating: null,
	servings: null,
	notes: null,
	usualTrackKey: null,
	tags: [],
	ingredients: [],
	instructions: [],
	imageUrl: null,
};

export interface CreateRecipeInput {
	name: string;
}

/** Saves a recipe by name only. It has no link, so `url` and `sourceDomain` stay empty. */
export function useCreateRecipe() {
	const queryClient = useQueryClient();

	return useMutation({
		mutationFn: ({ name }: CreateRecipeInput) => invoke<RecipeDetail>("recipe_create", {
			input: { ...blankRecipe, name },
		}),
		onSuccess: () => queryClient.invalidateQueries({ queryKey: ["recipes"] }),
	});
}

export interface ImportResult {
	recipe: RecipeDetail;
	/** Why the page could not be read, or null when it was imported. The link is saved either way. */
	importError: string | null;
}

/**
 * Saves a pasted link as a recipe. The page is read first, so the recipe gets its title, ingredients,
 * steps and image. If the page cannot be read, the link is saved with a title from the link and the
 * user fills in the rest on the edit page.
 */
export function useImportRecipeLink() {
	const queryClient = useQueryClient();

	return useMutation({
		mutationFn: async (link: string): Promise<ImportResult> => {
			let scraped: ScrapedRecipe;
			try {
				scraped = await invoke<ScrapedRecipe>("scrape_recipe_url", { url: link });
			} catch (error) {
				const recipe = await invoke<RecipeDetail>("recipe_create", {
					input: { ...blankRecipe, name: titleFromLink(link), url: link, sourceDomain: domainOf(link) },
				});
				return { recipe, importError: String(error) };
			}

			const recipe = await invoke<RecipeDetail>("recipe_create", {
				input: {
					...blankRecipe,
					name: scraped.name,
					url: scraped.sourceUrl,
					sourceDomain: domainOf(scraped.sourceUrl),
					ingredients: scraped.ingredients,
					instructions: scraped.instructions,
					imageUrl: scraped.imageUrl,
				},
			});
			return { recipe, importError: null };
		},
		onSuccess: () => queryClient.invalidateQueries({ queryKey: ["recipes"] }),
	});
}

/** Replaces every field of the recipe, including its tags. */
export function useUpdateRecipe(id: string) {
	const queryClient = useQueryClient();

	return useMutation({
		mutationFn: (input: RecipeInput) => invoke<RecipeDetail>("recipe_update", { id, input }),
		onSuccess: () => queryClient.invalidateQueries({ queryKey: ["recipes"] }),
	});
}

/** Deletes the recipe. Its planned meals go with it, so the schedule refetches too. */
export function useDeleteRecipe() {
	const queryClient = useQueryClient();

	return useMutation({
		mutationFn: (id: string) => invoke<void>("recipe_delete", { id }),
		onSuccess: () => Promise.all([
			queryClient.invalidateQueries({ queryKey: ["recipes"] }),
			queryClient.invalidateQueries({ queryKey: ["schedule"] }),
		]),
	});
}

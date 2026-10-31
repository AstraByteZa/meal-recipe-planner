import { describe, expect, it } from "vitest";
import { recipeToInput } from "./recipeInput";
import type { RecipeDetail } from "./types";

const stored: RecipeDetail = {
	id: "7",
	name: "Chicken tikka traybake",
	url: "https://example.com/tikka",
	sourceDomain: "example.com",
	tags: ["dinner", "tray"],
	rating: 4,
	lastUsedDaysAgo: 3,
	savedDaysAgo: 40,
	plannedCount: 2,
	minutes: 45,
	servings: 4,
	notes: "Less chilli next time",
	imageUrl: "https://example.com/tikka.jpg",
	ingredients: ["800 g chicken thighs", "150 g yoghurt"],
	instructions: ["Marinate the chicken.", "Roast for 25 minutes."],
	usualTrackKey: "dinner",
	history: [],
};

describe("recipeToInput", () => {
	it("carries every stored field into the save payload", () => {
		expect(recipeToInput(stored)).toEqual({
			name: "Chicken tikka traybake",
			url: "https://example.com/tikka",
			sourceDomain: "example.com",
			minutes: 45,
			rating: 4,
			servings: 4,
			notes: "Less chilli next time",
			usualTrackKey: "dinner",
			tags: ["dinner", "tray"],
			ingredients: ["800 g chicken thighs", "150 g yoghurt"],
			instructions: ["Marinate the chicken.", "Roast for 25 minutes."],
			imageUrl: "https://example.com/tikka.jpg",
		});
	});

	it("treats a missing rating as no rating", () => {
		const { rating: _rating, ...unrated } = stored;
		expect(recipeToInput({ ...unrated, rating: undefined }).rating).toBeNull();
	});
});

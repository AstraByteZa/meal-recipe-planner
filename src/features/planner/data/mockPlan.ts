// Placeholder data until gap suggestions are backed by real storage.
import type { GapSuggestion, Recipe } from "../types";

const recipes = {
  overnightOats: { id: "overnight-oats", name: "Overnight oats", minutes: 10, rating: 4 },
  greekYoghurtBowl: { id: "greek-yoghurt-bowl", name: "Greek yoghurt bowl", minutes: 5, rating: 4 },
  shakshuka: { id: "shakshuka", name: "Shakshuka", minutes: 25, rating: 5 },
  bananaPancakes: { id: "banana-pancakes", name: "Banana pancakes", minutes: 20, rating: 5 },
  spinachOmelette: { id: "spinach-omelette", name: "Spinach & feta omelette", minutes: 15, rating: 4 },
  chickpeaWrap: { id: "chickpea-wrap", name: "Chickpea salad wrap", minutes: 15, rating: 4 },
  salmonRiceBowl: { id: "salmon-rice-bowl", name: "Salmon rice bowl", minutes: 20, rating: 4 },
  lentilSoup: { id: "lentil-soup", name: "Lentil soup", minutes: 35, rating: 4 },
  caprese: { id: "caprese-toastie", name: "Caprese toastie", minutes: 10, rating: 3 },
  roastVeg: { id: "roast-veg-hummus", name: "Roast veg & hummus", minutes: 30, rating: 4 },
  sheetPanSalmon: { id: "sheet-pan-salmon", name: "Sheet-pan salmon", minutes: 30, rating: 4 },
  mushroomRisotto: { id: "mushroom-risotto", name: "Mushroom risotto", minutes: 45, rating: 4 },
  chickenTikka: { id: "chicken-tikka-traybake", name: "Chicken tikka traybake", minutes: 40, rating: 5 },
  beefChilli: { id: "beef-chilli", name: "Beef chilli", minutes: 60, rating: 5 },
  homemadePizza: { id: "homemade-pizza", name: "Homemade pizza", minutes: 50, rating: 5 },
  veggieLasagne: { id: "veggie-lasagne", name: "Veggie lasagne", minutes: 60, rating: 3 },
  missoAubergine: { id: "miso-aubergine-bowl", name: "Miso aubergine bowl", minutes: 40, rating: 5 },
  lemonOrzo: { id: "lemon-orzo-prawns", name: "Lemon orzo with prawns", minutes: 25 },
} satisfies Record<string, Recipe>;

export const mockGapSuggestions: GapSuggestion[] = [
  { recipe: recipes.missoAubergine, trackKey: "dinner", reason: "not planned in 6 weeks" },
  { recipe: recipes.veggieLasagne, trackKey: "dinner", reason: "last used 3 weeks ago" },
  { recipe: recipes.lemonOrzo, trackKey: "dinner", reason: "never planned" },
  { recipe: recipes.spinachOmelette, trackKey: "breakfast", reason: "last used 5 weeks ago" },
];

/** Dietary needs the household can pick. The keys must match the CHECK in migration 0004. */
export const DIETARY_NEEDS = [
	{ value: "vegetarian", label: "Vegetarian" },
	{ value: "vegan", label: "Vegan" },
	{ value: "pescatarian", label: "Pescatarian" },
	{ value: "gluten-free", label: "Gluten-free" },
	{ value: "dairy-free", label: "Dairy-free" },
	{ value: "nut-free", label: "Nut-free" },
] as const;

export const WEEK_START_OPTIONS = [
	{ value: "monday", label: "Monday" },
	{ value: "sunday", label: "Sunday" },
] as const;

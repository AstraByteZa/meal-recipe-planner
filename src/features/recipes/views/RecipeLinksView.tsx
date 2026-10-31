import { useState } from "react";
import { useSuspenseQuery } from "@tanstack/react-query";
import { useNavigate } from "@tanstack/react-router";
import { Button, Group, Text, TextInput, Title, UnstyledButton } from "@mantine/core";
import { IconSearch } from "@tabler/icons-react";
import { RecipeLinksTable } from "../components/RecipeLinksTable";
import { SaveLinkForm } from "../components/SaveLinkForm";
import { recipesQueryOptions } from "../queries";
import classes from "./RecipeLinksView.module.css";

const filters = ["All", "Breakfast", "Lunch", "Dinner", "Quick", "Vegetarian"];

export function RecipeLinksView() {
	const [search, setSearch] = useState("");
	const [activeFilter, setActiveFilter] = useState("All");
	const { data: recipes } = useSuspenseQuery(recipesQueryOptions);
	const navigate = useNavigate();

	const query = search.trim().toLowerCase();
	const visibleRecipes = recipes.filter((recipe) =>
		(activeFilter === "All" || recipe.tags.includes(activeFilter))
		&& (query === "" || recipe.name.toLowerCase().includes(query) || recipe.sourceDomain.toLowerCase().includes(query))
	);

	return <div className={classes.view}>
		<header className={classes.header}>
			<div>
				<Text size="xs" c="dimmed" className={classes.eyebrow}>Library</Text>
				<Title order={1}>Recipe links</Title>
			</div>
			<Text size="sm" c="dimmed">{recipes.length} saved</Text>
		</header>

		<section className={classes.panel}>
			{/* The edit page completes the title and the rest of the recipe. */}
			<SaveLinkForm onSaved={(recipe) => navigate({ to: "/recipes/$recipeId/edit", params: { recipeId: recipe.id } })} />
		</section>

		<section className={classes.panel}>
			<div className={classes.toolbar}>
				<div className={classes.searchRow}>
					<TextInput
						placeholder="Search by name or site"
						leftSection={<IconSearch size={16} stroke={1.5} />}
						aria-label="Search recipes"
						className={classes.search}
						value={search}
						onChange={(event) => setSearch(event.currentTarget.value)}
					/>
					<Button variant="default">Sort: Last used</Button>
				</div>

				<Group gap="xs">
					{filters.map((filter) => {
						const isActive = filter === activeFilter;

						return <UnstyledButton
							key={filter}
							className={isActive ? `${classes.chip} ${classes.chipActive}` : classes.chip}
							aria-pressed={isActive}
							onClick={() => setActiveFilter(filter)}
						>
							{filter}
						</UnstyledButton>
					})}
				</Group>
			</div>

			<RecipeLinksTable recipes={visibleRecipes} />
		</section>
	</div>
}

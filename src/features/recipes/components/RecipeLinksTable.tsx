import { IconPlus } from "@tabler/icons-react";
import { ActionIcon, Badge, Group, Text } from "@mantine/core";
import { Link } from "@tanstack/react-router";
import dayjs from "../../../lib/dayjs";
import { formatDaysAgo } from "../format";
import type { SavedRecipe } from "../types";
import { StarRating } from "./StarRating";
import classes from "./RecipeLinksTable.module.css";

interface RecipeLinksTableProps {
	recipes: SavedRecipe[];
}

export function RecipeLinksTable({ recipes }: RecipeLinksTableProps) {
	return <div className={classes.table}>
		<div className={classes.headerRow}>
			<Text size="xs" c="dimmed" className={classes.headerCell}>Recipe</Text>
			<Text size="xs" c="dimmed" className={classes.headerCell}>Tags</Text>
			<Text size="xs" c="dimmed" className={classes.headerCell}>Rating</Text>
			<Text size="xs" c="dimmed" className={classes.headerCell}>Last used in a plan</Text>
			<Text size="xs" c="dimmed" className={classes.headerCell}>Planned</Text>
			<div />
		</div>

		{recipes.length === 0 && <Text size="sm" c="dimmed" className={classes.empty}>No recipes match your search or filter.</Text>}

		{recipes.map((recipe) => <RecipeRow key={recipe.id} recipe={recipe} />)}
	</div>
}

interface RecipeRowProps {
	recipe: SavedRecipe;
}

function RecipeRow({ recipe }: RecipeRowProps) {
	const isToday = recipe.lastUsedDaysAgo === 0;

	return <div className={classes.row}>
		<Link
			to="/recipes/$recipeId"
			params={{ recipeId: recipe.id }}
			className={`${classes.recipe} ${classes.recipeLink}`}
		>
			<div className={classes.avatar}>{recipe.name.charAt(0)}</div>
			<div className={classes.recipeText}>
				<Text size="sm" fw={600} className={classes.recipeName}>{recipe.name}</Text>
				<Text size="xs" c="dimmed">{recipe.sourceDomain || "No link yet"}</Text>
			</div>
		</Link>

		<Group gap={6} wrap="nowrap">
			{recipe.tags.map((tag) => <Badge key={tag} variant="default" radius="sm" size="sm">{tag}</Badge>)}
		</Group>

		<StarRating value={recipe.rating} />

		<div>
			{recipe.lastUsedDaysAgo === null ? <>
				<Text size="sm">Never</Text>
				<Text size="xs" c="dimmed">Saved {formatDaysAgo(recipe.savedDaysAgo)}</Text>
			</> : <>
				<Group gap={6}>
					<Text size="sm">{dayjs().subtract(recipe.lastUsedDaysAgo, "day").format("ddd D MMM")}</Text>
					{isToday && <Badge size="xs" variant="filled" radius="sm">Today</Badge>}
				</Group>
				<Text size="xs" c="dimmed">{isToday ? "In this week's plan" : formatDaysAgo(recipe.lastUsedDaysAgo)}</Text>
			</>}
		</div>

		<Text size="sm">{recipe.plannedCount}×</Text>

		<ActionIcon variant="default" size="lg" aria-label={`Add ${recipe.name} to plan`}>
			<IconPlus size={16} stroke={1.5} />
		</ActionIcon>
	</div>
}

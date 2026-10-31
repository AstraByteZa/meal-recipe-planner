import { IconCheck, IconPlus } from "@tabler/icons-react";
import { useSuspenseQuery } from "@tanstack/react-query";
import { ActionIcon, Badge, Stack, Text } from "@mantine/core";
import { SaveLinkForm } from "../../recipes/components/SaveLinkForm";
import { StarRating } from "../../recipes/components/StarRating";
import { useCreateRecipe } from "../../recipes/mutations";
import { recipesQueryOptions } from "../../recipes/queries";
import { StepFooter } from "./StepFooter";
import { StepIntro } from "./StepIntro";
import classes from "./FirstRecipeStep.module.css";

/** Starter recipes offered by name. They have no link yet, so no source is shown. */
const starterIdeas = ["Mushroom risotto", "Sheet-pan salmon"];

interface FirstRecipeStepProps {
	onBack: () => void;
	onNext: () => void;
	onSkip: () => void;
}

export function FirstRecipeStep({ onBack, onNext, onSkip }: FirstRecipeStepProps) {
	const { data: recipes } = useSuspenseQuery(recipesQueryOptions);
	const createRecipe = useCreateRecipe();

	const isSaved = (name: string) => recipes.some((recipe) => recipe.name.toLowerCase() === name.toLowerCase());

	return <>
		<StepIntro
			step={4}
			title="Save your first recipe link"
			description="Paste any recipe link. You can rate it and track when you last used it."
		/>

		<SaveLinkForm />

		<Stack gap="sm">
			<Text size="xs" c="dimmed" className={classes.label}>Saved so far</Text>

			{recipes.length === 0
				? <Text size="sm" c="dimmed">Nothing saved yet.</Text>
				: recipes.map((recipe) => <div key={recipe.id} className={classes.row}>
					<span className={classes.avatar}>{recipe.name.charAt(0).toUpperCase()}</span>
					<div className={classes.text}>
						<Text fw={600}>{recipe.name}</Text>
						<Text size="sm" c="dimmed">{recipe.sourceDomain || "No link yet"}</Text>
					</div>
					{recipe.rating != null && <StarRating value={recipe.rating} />}
					<Badge variant="light" size="sm">Saved</Badge>
				</div>)}
		</Stack>

		<Stack gap="sm">
			<Text size="xs" c="dimmed" className={classes.label}>Or start with a few ideas</Text>

			{starterIdeas.map((name) => {
				const saved = isSaved(name);

				return <div key={name} className={classes.row}>
					<span className={classes.avatar}>{name.charAt(0)}</span>
					<div className={classes.text}>
						<Text fw={600}>{name}</Text>
						<Text size="sm" c="dimmed">Starter idea</Text>
					</div>
					<ActionIcon
						variant="default"
						size={36}
						aria-label={saved ? `${name} is saved` : `Save ${name}`}
						disabled={saved || createRecipe.isPending}
						onClick={() => createRecipe.mutate({ name })}
					>
						{saved ? <IconCheck size={16} stroke={2} /> : <IconPlus size={16} stroke={1.5} />}
					</ActionIcon>
				</div>
			})}
		</Stack>

		<StepFooter onBack={onBack} onSkip={onSkip} onNext={onNext} />
	</>
}

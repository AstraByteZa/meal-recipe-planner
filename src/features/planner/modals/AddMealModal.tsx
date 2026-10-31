import { useState } from "react";
import { useForm } from "@tanstack/react-form";
import { useQuery } from "@tanstack/react-query";
import { modals, type ContextModalProps } from "@mantine/modals";
import { Alert, Button, Group, Select, Stack, TextInput } from "@mantine/core";
import { z } from "zod";
import { recipesQueryOptions } from "../../recipes/queries";
import { useCreateRecipe } from "../../recipes/mutations";
import { useClearScheduleEntry, useSetScheduleEntry } from "../mutations";

const addMealSchema = z.object({
	recipeId: z.string().min(1, "Choose a recipe"),
});

/** The schedule slot a meal is added to or edited in. */
export interface AddMealSlot {
	date: string;
	trackKey: string;
	/** The recipe already in the slot. Set when editing, unset when adding. */
	recipeId?: string;
}

export function openAddMealModal({ title, ...slot }: AddMealSlot & { title: string }) {
	modals.openContextModal({ modal: "addMeal", title, innerProps: slot });
}

export function AddMealModal({ context, id, innerProps }: ContextModalProps<AddMealSlot>) {
	const { data: recipes = [], isError, error } = useQuery(recipesQueryOptions);
	const [newName, setNewName] = useState("");
	const createRecipe = useCreateRecipe();
	const setEntry = useSetScheduleEntry();
	const clearEntry = useClearScheduleEntry();
	const isEditing = innerProps.recipeId !== undefined;

	const form = useForm({
		defaultValues: { recipeId: innerProps.recipeId ?? "" },
		validators: { onChange: addMealSchema },
		onSubmit: ({ value }) => {
			setEntry.mutate(
				{ date: innerProps.date, trackKey: innerProps.trackKey, recipeId: value.recipeId },
				{ onSuccess: () => context.closeModal(id) },
			);
		},
	});

	// The new recipe is selected in the form. The user still saves the meal.
	const addNewRecipe = () => {
		const name = newName.trim();
		if (!name) return;

		createRecipe.mutate({ name }, {
			onSuccess: (recipe) => {
				form.setFieldValue("recipeId", recipe.id);
				setNewName("");
			},
		});
	};

	return <form onSubmit={(event) => {
		event.preventDefault();
		form.handleSubmit();
	}}>
		<Stack gap="md">
			{isError && <Alert color="red" title="Could not load recipes">{String(error)}</Alert>}

			<form.Field name="recipeId">
				{(field) => <Select
					label="Recipe"
					placeholder="Choose a saved recipe"
					searchable
					nothingFoundMessage="No recipes saved yet"
					data={recipes.map((recipe) => ({ value: recipe.id, label: recipe.name }))}
					value={field.state.value}
					onChange={(value) => field.handleChange(value ?? "")}
					onBlur={field.handleBlur}
					error={field.state.meta.isTouched && field.state.meta.errors.length > 0
						? field.state.meta.errors.map((issue) => issue?.message).join(", ")
						: undefined}
				/>}
			</form.Field>

			<Group align="flex-end" gap="xs" wrap="nowrap">
				<TextInput
					label="Or add a new recipe by name"
					placeholder="e.g. Shakshuka"
					value={newName}
					onChange={(event) => setNewName(event.currentTarget.value)}
					onKeyDown={(event) => {
						if (event.key !== "Enter") return;
						event.preventDefault();
						addNewRecipe();
					}}
					flex={1}
				/>
				<Button variant="default" onClick={addNewRecipe} disabled={!newName.trim()} loading={createRecipe.isPending}>
					Add recipe
				</Button>
			</Group>

			{createRecipe.isError && <Alert color="red" title="Could not add the recipe">{String(createRecipe.error)}</Alert>}
			{setEntry.isError && <Alert color="red" title="Could not save the meal">{String(setEntry.error)}</Alert>}
			{clearEntry.isError && <Alert color="red" title="Could not remove the meal">{String(clearEntry.error)}</Alert>}

			<Group justify="space-between">
				{isEditing
					? <Button
						variant="subtle"
						color="red"
						loading={clearEntry.isPending}
						onClick={() => clearEntry.mutate(
							{ date: innerProps.date, trackKey: innerProps.trackKey },
							{ onSuccess: () => context.closeModal(id) },
						)}
					>
						Remove meal
					</Button>
					: <div />}

				<Group gap="sm">
					<Button variant="default" onClick={() => context.closeModal(id)}>Cancel</Button>
					<Button type="submit" loading={setEntry.isPending}>Save meal</Button>
				</Group>
			</Group>
		</Stack>
	</form>
}

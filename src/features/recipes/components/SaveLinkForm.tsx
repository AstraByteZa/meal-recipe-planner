import { useForm } from "@tanstack/react-form";
import { IconLink } from "@tabler/icons-react";
import { Alert, Button, Group, Loader, Text, TextInput } from "@mantine/core";
import { notifications } from "@mantine/notifications";
import { z } from "zod";
import { normalizeLink } from "../format";
import { useImportRecipeLink } from "../mutations";
import type { RecipeDetail } from "../types";
import classes from "./SaveLinkForm.module.css";

const linkSchema = z.object({
	link: z.string().refine((value) => normalizeLink(value) !== null, "Paste a web link, like https://example.com/recipe"),
});

interface SaveLinkFormProps {
	/** Called with the saved recipe after the form has cleared. */
	onSaved?: (recipe: RecipeDetail) => void;
}

/** Paste a link to import it as a recipe. Used by the recipe links page and onboarding. */
export function SaveLinkForm({ onSaved }: SaveLinkFormProps) {
	const saveLink = useImportRecipeLink();

	const linkForm = useForm({
		defaultValues: { link: "" },
		validators: { onChange: linkSchema },
		onSubmit: ({ value }) => {
			const link = normalizeLink(value.link);
			if (!link) return;

			// The caller decides what comes next. A failed import still saves the link.
			saveLink.mutate(link, {
				onSuccess: ({ recipe, importError }) => {
					linkForm.reset();
					if (importError) {
						notifications.show({
							color: "yellow",
							title: "Couldn't import the recipe",
							message: `${importError} The link is saved, so you can fill in the details yourself.`,
							autoClose: 8000,
						});
					}
					onSaved?.(recipe);
				},
			});
		},
	});

	return <>
		<form onSubmit={(event) => {
			event.preventDefault();
			linkForm.handleSubmit();
		}}>
			<Group gap="sm" wrap="nowrap" align="flex-start">
				<linkForm.Field name="link">
					{(field) => <TextInput
						placeholder="Paste a recipe link to save it"
						leftSection={<IconLink size={16} stroke={1.5} />}
						aria-label="Recipe link"
						className={classes.linkInput}
						value={field.state.value}
						onChange={(event) => field.handleChange(event.currentTarget.value)}
						onBlur={field.handleBlur}
						disabled={saveLink.isPending}
						error={field.state.meta.isTouched && field.state.meta.errors.length > 0
							? field.state.meta.errors.map((issue) => issue?.message).join(", ")
							: undefined}
					/>}
				</linkForm.Field>
				<Button type="submit" loading={saveLink.isPending}>Save link</Button>
			</Group>
		</form>

		{saveLink.isPending && <Group gap="xs" mt="sm" role="status">
			<Loader size="xs" />
			<Text size="sm" c="dimmed">Scraping recipe from web...</Text>
		</Group>}

		{saveLink.isError && <Alert color="red" title="Could not save the link">{String(saveLink.error)}</Alert>}
	</>
}

import { useForm } from "@tanstack/react-form";
import { Alert, Button, Group, Text, Textarea, Title } from "@mantine/core";
import { z } from "zod";
import { StarInput } from "../StarRating";
import classes from "./CookDone.module.css";

const doneSchema = z.object({
	rating: z.number().int().min(1).max(5).nullable(),
	notes: z.string(),
});

export type CookDoneValues = z.infer<typeof doneSchema>;

interface CookDoneProps {
	recipeName: string;
	initial: CookDoneValues;
	saving: boolean;
	error: string | null;
	/** `changed` is false when the rating and notes are the same as before, so nothing needs saving. */
	onFinish: (values: CookDoneValues, changed: boolean) => void;
	onBackToSteps: () => void;
}

export function CookDone({ recipeName, initial, saving, error, onFinish, onBackToSteps }: CookDoneProps) {
	const form = useForm({
		defaultValues: initial,
		validators: { onChange: doneSchema },
		onSubmit: ({ value }) => onFinish({ rating: value.rating, notes: value.notes.trim() }, form.state.isDirty),
	});

	return <form className={classes.panel} onSubmit={(event) => {
		event.preventDefault();
		void form.handleSubmit();
	}}>
		<Text className={classes.label}>All steps done</Text>
		<Title order={2}>How did {recipeName} turn out?</Title>

		<div>
			<Text size="sm" fw={500} mb={6}>Rating</Text>
			<form.Field name="rating">
				{(field) => <StarInput value={field.state.value} onChange={(rating) => field.handleChange(rating)} />}
			</form.Field>
		</div>

		<form.Field name="notes">
			{(field) => <Textarea
				label="Notes"
				description="What you would change next time"
				autosize
				minRows={3}
				value={field.state.value}
				onChange={(event) => field.handleChange(event.currentTarget.value)}
				onBlur={field.handleBlur}
			/>}
		</form.Field>

		{error && <Alert color="red" title="Could not save the recipe">{error}</Alert>}

		<Group justify="space-between">
			<Button variant="default" onClick={onBackToSteps} disabled={saving}>Back to last step</Button>
			<Button type="submit" loading={saving}>Save and finish</Button>
		</Group>
	</form>;
}

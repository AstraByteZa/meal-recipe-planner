import { useForm } from "@tanstack/react-form";
import { Button, Group, Stack, Text, TextInput } from "@mantine/core";
import { InviteCodeInput } from "./InviteCodeInput";
import { fieldError } from "./formErrors";
import { useFamilyCreate, useFamilyJoin } from "../mutations";
import { familyJoinSchema, familyNameSchema } from "../schema";
import classes from "./FamilyStartCards.module.css";

/** Create or join a family. Shown in the sync modal and on the Family page, to devices with no family. */
export function FamilyStartCards() {
	return <Stack gap="md">
		<CreateFamilyCard />
		<JoinFamilyCard />
	</Stack>;
}

export function CreateFamilyCard() {
	const create = useFamilyCreate();
	const form = useForm({
		defaultValues: { name: "" },
		validators: { onChange: familyNameSchema },
		onSubmit: ({ value }) => {
			create.mutate({ name: value.name.trim() }, { onSuccess: () => form.reset() });
		},
	});

	return <div className={classes.card}>
		<Stack gap={2}>
			<Text fw={600}>Create a family</Text>
			<Text size="sm" c="dimmed">Share recipes and plans with people you cook with. You become the family owner.</Text>
		</Stack>
		<form onSubmit={(event) => {
			event.preventDefault();
			form.handleSubmit();
		}}>
			<Stack gap="sm">
				<form.Field name="name">
					{(field) => <TextInput
						label="Family name"
						placeholder="e.g. The Joostes"
						value={field.state.value}
						onChange={(event) => field.handleChange(event.currentTarget.value)}
						onBlur={field.handleBlur}
						error={fieldError(field) ?? (create.isError ? String(create.error) : undefined)}
					/>}
				</form.Field>
				<Group justify="flex-end">
					<Button type="submit" loading={create.isPending}>Create family</Button>
				</Group>
			</Stack>
		</form>
	</div>;
}

export function JoinFamilyCard() {
	const join = useFamilyJoin();
	const form = useForm({
		defaultValues: { inviteCode: "" },
		validators: { onChange: familyJoinSchema },
		onSubmit: ({ value }) => {
			join.mutate({ inviteCode: value.inviteCode }, { onSuccess: () => form.reset() });
		},
	});

	return <div className={classes.card}>
		<Stack gap={2}>
			<Text fw={600}>Join a family</Text>
			<Text size="sm" c="dimmed">Ask the family owner for an invite code.</Text>
		</Stack>
		<form onSubmit={(event) => {
			event.preventDefault();
			form.handleSubmit();
		}}>
			<Stack gap="sm">
				<form.Field name="inviteCode">
					{(field) => <InviteCodeInput
						label="Invite code"
						placeholder="XXXX-XXXX"
						value={field.state.value}
						onChange={(value) => field.handleChange(value)}
						onBlur={field.handleBlur}
						error={fieldError(field) ?? (join.isError ? String(join.error) : undefined)}
					/>}
				</form.Field>
				<Group justify="flex-end">
					<Button type="submit" variant="default" loading={join.isPending}>Join family</Button>
				</Group>
			</Stack>
		</form>
	</div>;
}

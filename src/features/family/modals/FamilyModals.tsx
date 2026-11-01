import type { ReactNode } from "react";
import { useForm } from "@tanstack/react-form";
import { modals, type ContextModalProps } from "@mantine/modals";
import { Alert, Button, Group, Stack, Text, TextInput } from "@mantine/core";
import { z } from "zod";
import { fieldError } from "../../sync/components/formErrors";
import { useFamilyDelete, useFamilyMemberRemove, useFamilyRename, useFamilyTransfer } from "../../sync/mutations";
import { familyNameSchema } from "../../sync/schema";

// Each modal is opened by name through its helper. Every write closes the modal on success and
// shows the server's message in place if it fails.

export function openRenameFamilyModal(name: string) {
	modals.openContextModal({ modal: "renameFamily", title: "Rename family", innerProps: { name } });
}

export function openConfirmRemoveMemberModal(props: { userId: string; name: string; familyName: string }) {
	modals.openContextModal({ modal: "confirmRemoveMember", title: "Remove from family", innerProps: props });
}

export function openConfirmTransferModal(props: { userId: string; name: string; familyName: string }) {
	modals.openContextModal({ modal: "confirmTransfer", title: "Make owner", innerProps: props });
}

export function openConfirmDeleteModal(props: { familyName: string }) {
	modals.openContextModal({ modal: "confirmDelete", title: "Delete family", innerProps: props });
}

export function openConfirmLeaveModal(props: { userId: string; familyName: string }) {
	modals.openContextModal({ modal: "confirmLeave", title: "Leave family", innerProps: props });
}

function errorText(error: unknown): string | undefined {
	return error ? String(error) : undefined;
}

export function RenameFamilyModal({ context, id, innerProps }: ContextModalProps<{ name: string }>) {
	const rename = useFamilyRename();
	const form = useForm({
		defaultValues: { name: innerProps.name },
		validators: { onChange: familyNameSchema },
		onSubmit: ({ value }) => {
			rename.mutate({ name: value.name.trim() }, { onSuccess: () => context.closeModal(id) });
		},
	});

	return <form onSubmit={(event) => {
		event.preventDefault();
		form.handleSubmit();
	}}>
		<Stack gap="md">
			<form.Field name="name">
				{(field) => <TextInput
					label="Family name"
					value={field.state.value}
					onChange={(event) => field.handleChange(event.currentTarget.value)}
					onBlur={field.handleBlur}
					error={fieldError(field)}
				/>}
			</form.Field>
			{rename.isError && <Alert color="red" title="Could not rename the family">{errorText(rename.error)}</Alert>}
			<Group justify="flex-end">
				<Button variant="default" onClick={() => context.closeModal(id)}>Cancel</Button>
				<Button type="submit" loading={rename.isPending}>Save</Button>
			</Group>
		</Stack>
	</form>;
}

export function ConfirmRemoveMemberModal({ context, id, innerProps }: ContextModalProps<{ userId: string; name: string; familyName: string }>) {
	const remove = useFamilyMemberRemove();

	return <ConfirmShell
		error={remove.isError ? { title: "Could not remove them", message: errorText(remove.error) } : undefined}
		confirmLabel="Remove"
		loading={remove.isPending}
		onCancel={() => context.closeModal(id)}
		onConfirm={() => remove.mutate({ userId: innerProps.userId }, { onSuccess: () => context.closeModal(id) })}
	>
		Remove {innerProps.name} from {innerProps.familyName}? Their copy of the family's recipes stays on their
		device as private data. They can rejoin with a new invite code.
	</ConfirmShell>;
}

export function ConfirmTransferModal({ context, id, innerProps }: ContextModalProps<{ userId: string; name: string; familyName: string }>) {
	const transfer = useFamilyTransfer();

	return <ConfirmShell
		error={transfer.isError ? { title: "Could not change the owner", message: errorText(transfer.error) } : undefined}
		confirmLabel="Make owner"
		loading={transfer.isPending}
		onCancel={() => context.closeModal(id)}
		onConfirm={() => transfer.mutate({ userId: innerProps.userId }, { onSuccess: () => context.closeModal(id) })}
	>
		Make {innerProps.name} the owner of {innerProps.familyName}? You will become a regular member.
	</ConfirmShell>;
}

export function ConfirmLeaveModal({ context, id, innerProps }: ContextModalProps<{ userId: string; familyName: string }>) {
	const leave = useFamilyMemberRemove();

	return <ConfirmShell
		error={leave.isError ? { title: "Could not leave the family", message: errorText(leave.error) } : undefined}
		confirmLabel="Leave family"
		loading={leave.isPending}
		onCancel={() => context.closeModal(id)}
		onConfirm={() => leave.mutate({ userId: innerProps.userId }, { onSuccess: () => context.closeModal(id) })}
	>
		Leave {innerProps.familyName}? Your copy of the family's recipes stays on this device as private data.
		You can rejoin with a new invite code.
	</ConfirmShell>;
}

/** Deleting asks for the family name, so it cannot happen by accident. */
export function ConfirmDeleteModal({ context, id, innerProps }: ContextModalProps<{ familyName: string }>) {
	const del = useFamilyDelete();
	const { familyName } = innerProps;
	const form = useForm({
		defaultValues: { confirmName: "" },
		validators: {
			onChange: z.object({
				confirmName: z.string().refine((value) => value === familyName, `Type ${familyName} exactly to confirm`),
			}),
		},
		onSubmit: () => {
			del.mutate({}, { onSuccess: () => context.closeModal(id) });
		},
	});

	return <form onSubmit={(event) => {
		event.preventDefault();
		form.handleSubmit();
	}}>
		<Stack gap="md">
			<Text size="sm">
				Delete {familyName} for everyone? Members lose the shared family and keep their copies of the
				recipes as private data. This cannot be undone.
			</Text>
			<form.Field name="confirmName">
				{(field) => <TextInput
					label={`Type ${familyName} to confirm`}
					value={field.state.value}
					onChange={(event) => field.handleChange(event.currentTarget.value)}
					onBlur={field.handleBlur}
					error={fieldError(field)}
				/>}
			</form.Field>
			{del.isError && <Alert color="red" title="Could not delete the family">{errorText(del.error)}</Alert>}
			<Group justify="flex-end">
				<Button variant="default" onClick={() => context.closeModal(id)}>Cancel</Button>
				<form.Subscribe selector={(state) => state.values.confirmName === familyName}>
					{(matches) => <Button color="red" type="submit" disabled={!matches} loading={del.isPending}>
						Delete family
					</Button>}
				</form.Subscribe>
			</Group>
		</Stack>
	</form>;
}

/** Shared layout for the confirm dialogs: a message, an optional error, and Cancel / confirm. */
function ConfirmShell({
	children,
	error,
	confirmLabel,
	loading,
	onCancel,
	onConfirm,
}: {
	children: ReactNode;
	error?: { title: string; message?: string };
	confirmLabel: string;
	loading: boolean;
	onCancel: () => void;
	onConfirm: () => void;
}) {
	return <Stack gap="md">
		<Text size="sm">{children}</Text>
		{error && <Alert color="red" title={error.title}>{error.message}</Alert>}
		<Group justify="flex-end">
			<Button variant="default" onClick={onCancel}>Cancel</Button>
			<Button color="red" onClick={onConfirm} loading={loading}>{confirmLabel}</Button>
		</Group>
	</Stack>;
}

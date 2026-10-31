import { useState } from "react";
import { useForm } from "@tanstack/react-form";
import { useQuery } from "@tanstack/react-query";
import { modals, type ContextModalProps } from "@mantine/modals";
import {
	Alert,
	Button,
	Code,
	CopyButton,
	Group,
	PasswordInput,
	SegmentedControl,
	Stack,
	Text,
	TextInput,
} from "@mantine/core";
import dayjs from "../../../lib/dayjs";
import { syncStatusQueryOptions } from "../queries";
import {
	useFamilyCreate,
	useFamilyJoin,
	useSyncLogin,
	useSyncLogout,
	useSyncNow,
	useSyncRegister,
} from "../mutations";
import { accountFormSchema, familyCreateSchema, familyJoinSchema, type SyncStatus } from "../schema";

export function openSyncSettingsModal() {
	modals.openContextModal({ modal: "syncSettings", title: "Sync", innerProps: {} });
}

export function SyncSettingsModal(_props: ContextModalProps) {
	const { data: status, isError, error } = useQuery(syncStatusQueryOptions);
	const syncNow = useSyncNow();
	const logout = useSyncLogout();

	if (!status) {
		return isError ? <Alert color="red" title="Could not load sync">{String(error)}</Alert> : null;
	}

	return <Stack gap="md">
		<StatusSummary status={status} />

		{syncNow.isError && <Alert color="red" title="Could not sync">{String(syncNow.error)}</Alert>}
		{logout.isError && <Alert color="red" title="Could not sign out">{String(logout.error)}</Alert>}

		{status.signedIn
			? <Group justify="space-between">
				<Button variant="default" onClick={() => syncNow.mutate({})} loading={syncNow.isPending} disabled={status.syncing}>
					Sync now
				</Button>
				<Button variant="subtle" color="red" onClick={() => logout.mutate({})} loading={logout.isPending}>
					Sign out
				</Button>
			</Group>
			: <AccountForm initialServerUrl={status.serverUrl ?? ""} />}

		{status.signedIn && <FamilySection family={status.family} />}
	</Stack>;
}

function StatusSummary({ status }: { status: SyncStatus }) {
	const lastSynced = status.lastSyncedAt
		? dayjs(status.lastSyncedAt).format("D MMM YYYY, HH:mm")
		: "Not synced yet";

	return <Stack gap={4}>
		<Text fw={500}>{status.signedIn ? `Signed in as ${status.email}` : "Not signed in"}</Text>
		<Text size="sm" c="dimmed">
			{status.signedIn
				? `Server: ${status.serverUrl}`
				: "Sign in to sync this device. Until then, your data stays here."}
		</Text>
		{status.signedIn && <>
			<Text size="sm">
				{status.syncing ? "Syncing…" : `${status.pending} ${status.pending === 1 ? "change" : "changes"} waiting to upload`}
			</Text>
			<Text size="sm" c="dimmed">Last synced: {lastSynced}</Text>
		</>}
		{status.lastError && <Alert color="red" title="Last sync failed" mt="xs">{status.lastError}</Alert>}
	</Stack>;
}

function AccountForm({ initialServerUrl }: { initialServerUrl: string }) {
	const [mode, setMode] = useState<"signIn" | "register">("signIn");
	const login = useSyncLogin();
	const register = useSyncRegister();
	const attempt = mode === "register" ? register : login;

	const form = useForm({
		defaultValues: { serverUrl: initialServerUrl, email: "", password: "" },
		validators: { onChange: accountFormSchema },
		onSubmit: ({ value }) => {
			attempt.mutate(
				{ serverUrl: value.serverUrl.trim(), email: value.email.trim(), password: value.password },
				{ onSuccess: () => form.setFieldValue("password", "") },
			);
		},
	});

	return <form onSubmit={(event) => {
		event.preventDefault();
		form.handleSubmit();
	}}>
		<Stack gap="sm">
			<SegmentedControl
				value={mode}
				onChange={(value) => setMode(value as "signIn" | "register")}
				data={[
					{ label: "Sign in", value: "signIn" },
					{ label: "Create account", value: "register" },
				]}
			/>

			<form.Field name="serverUrl">
				{(field) => <TextInput
					label="Server"
					placeholder="https://sync.example.com"
					value={field.state.value}
					onChange={(event) => field.handleChange(event.currentTarget.value)}
					onBlur={field.handleBlur}
					error={field.state.meta.isTouched && field.state.meta.errors.length > 0
						? field.state.meta.errors.map((issue) => issue?.message).join(", ")
						: undefined}
				/>}
			</form.Field>

			<form.Field name="email">
				{(field) => <TextInput
					label="Email"
					type="email"
					autoComplete="email"
					value={field.state.value}
					onChange={(event) => field.handleChange(event.currentTarget.value)}
					onBlur={field.handleBlur}
					error={field.state.meta.isTouched && field.state.meta.errors.length > 0
						? field.state.meta.errors.map((issue) => issue?.message).join(", ")
						: undefined}
				/>}
			</form.Field>

			<form.Field name="password">
				{(field) => <PasswordInput
					label="Password"
					autoComplete={mode === "register" ? "new-password" : "current-password"}
					value={field.state.value}
					onChange={(event) => field.handleChange(event.currentTarget.value)}
					onBlur={field.handleBlur}
					error={field.state.meta.isTouched && field.state.meta.errors.length > 0
						? field.state.meta.errors.map((issue) => issue?.message).join(", ")
						: undefined}
				/>}
			</form.Field>

			{attempt.isError && <Alert color="red" title={mode === "register" ? "Could not create the account" : "Could not sign in"}>
				{String(attempt.error)}
			</Alert>}

			<Group justify="flex-end">
				<Button type="submit" loading={attempt.isPending}>
					{mode === "register" ? "Create account" : "Sign in"}
				</Button>
			</Group>
		</Stack>
	</form>;
}

function FamilySection({ family }: { family: SyncStatus["family"] }) {
	if (family) {
		return <Stack gap="xs">
			<Text fw={500}>Family: {family.name}</Text>
			<Text size="sm" c="dimmed">Recipes and meal plans added from now on are shared with the family.</Text>
			<Group gap="xs">
				<Text size="sm">Invite code</Text>
				<Code>{family.inviteCode}</Code>
				<CopyButton value={family.inviteCode}>
					{({ copied, copy }) => <Button size="xs" variant="default" onClick={copy}>
						{copied ? "Copied" : "Copy"}
					</Button>}
				</CopyButton>
			</Group>
		</Stack>;
	}

	return <Stack gap="md">
		<CreateFamilyForm />
		<JoinFamilyForm />
	</Stack>;
}

function CreateFamilyForm() {
	const create = useFamilyCreate();
	const form = useForm({
		defaultValues: { name: "" },
		validators: { onChange: familyCreateSchema },
		onSubmit: ({ value }) => {
			create.mutate({ name: value.name.trim() }, { onSuccess: () => form.reset() });
		},
	});

	return <form onSubmit={(event) => {
		event.preventDefault();
		form.handleSubmit();
	}}>
		<Stack gap="sm">
			<form.Field name="name">
				{(field) => <TextInput
					label="Create a family"
					placeholder="e.g. The Joostes"
					value={field.state.value}
					onChange={(event) => field.handleChange(event.currentTarget.value)}
					onBlur={field.handleBlur}
					error={field.state.meta.isTouched && field.state.meta.errors.length > 0
						? field.state.meta.errors.map((issue) => issue?.message).join(", ")
						: undefined}
				/>}
			</form.Field>
			{create.isError && <Alert color="red" title="Could not create the family">{String(create.error)}</Alert>}
			<Group justify="flex-end">
				<Button type="submit" variant="default" loading={create.isPending}>Create family</Button>
			</Group>
		</Stack>
	</form>;
}

function JoinFamilyForm() {
	const join = useFamilyJoin();
	const form = useForm({
		defaultValues: { inviteCode: "" },
		validators: { onChange: familyJoinSchema },
		onSubmit: ({ value }) => {
			join.mutate({ inviteCode: value.inviteCode.trim() }, { onSuccess: () => form.reset() });
		},
	});

	return <form onSubmit={(event) => {
		event.preventDefault();
		form.handleSubmit();
	}}>
		<Stack gap="sm">
			<form.Field name="inviteCode">
				{(field) => <TextInput
					label="Or join with an invite code"
					value={field.state.value}
					onChange={(event) => field.handleChange(event.currentTarget.value)}
					onBlur={field.handleBlur}
					error={field.state.meta.isTouched && field.state.meta.errors.length > 0
						? field.state.meta.errors.map((issue) => issue?.message).join(", ")
						: undefined}
				/>}
			</form.Field>
			{join.isError && <Alert color="red" title="Could not join the family">{String(join.error)}</Alert>}
			<Group justify="flex-end">
				<Button type="submit" variant="default" loading={join.isPending}>Join family</Button>
			</Group>
		</Stack>
	</form>;
}

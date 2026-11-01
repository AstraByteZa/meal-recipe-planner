import { useForm } from "@tanstack/react-form";
import { useQuery } from "@tanstack/react-query";
import { modals, type ContextModalProps } from "@mantine/modals";
import { Alert, Button, Group, Stack, Text, TextInput } from "@mantine/core";
import dayjs from "../../../lib/dayjs";
import { AccountForm } from "../components/AccountForm";
import { InviteCodeInput } from "../components/InviteCodeInput";
import { fieldError } from "../components/formErrors";
import { syncStatusQueryOptions } from "../queries";
import { useFamilyCreate, useFamilyJoin, useSyncLogout, useSyncNow } from "../mutations";
import { familyCreateSchema, familyJoinSchema, type SyncStatus } from "../schema";
import classes from "./SyncSettingsModal.module.css";

export function openSyncSettingsModal() {
	modals.openContextModal({ modal: "syncSettings", title: "Sync", innerProps: {} });
}

export function SyncSettingsModal(_props: ContextModalProps) {
	const { data: status, isError, error } = useQuery(syncStatusQueryOptions);

	if (!status) {
		return isError ? <Alert color="red" title="Could not load sync">{String(error)}</Alert> : null;
	}

	if (!status.signedIn) {
		return <Stack gap="md">
			<Text size="sm" c="dimmed">
				Sign in to sync this device. Until then, your data stays here.
			</Text>
			{status.lastError && <Alert color="red" title="Last sync failed">{status.lastError}</Alert>}
			<AccountForm initialServerUrl={status.serverUrl ?? status.defaultServerUrl ?? ""} />
		</Stack>;
	}

	return <Stack gap="md">
		<AccountCard status={status} />
		<SyncRow status={status} />

		{status.family
			? <div className={classes.card}>
				<Text fw={600}>Family: {status.family.name}</Text>
				<Text size="sm" c="dimmed">
					{status.family.role === "owner"
						? "You own this family. Manage members on the Family page."
						: "Recipes and meal plans added from now on are shared with the family."}
				</Text>
			</div>
			: <>
				<CreateFamilyCard />
				<JoinFamilyCard />
			</>}
	</Stack>;
}

function hostOf(url: string | null) {
	if (!url) return "";
	try {
		return new URL(url).host;
	} catch {
		return url;
	}
}

/** The signed-in account, with Sign out. */
function AccountCard({ status }: { status: SyncStatus }) {
	const logout = useSyncLogout();
	const name = status.user?.name || status.user?.email || "";

	return <div className={classes.account}>
		<div className={classes.accountRow}>
			<span className={classes.avatar}>{name.slice(0, 1).toUpperCase()}</span>
			<Stack gap={0} className={classes.grow}>
				<Text fw={600} truncate>{status.user?.email}</Text>
				<Text size="xs" c="dimmed" truncate>Signed in · {hostOf(status.serverUrl)}</Text>
			</Stack>
			<Button variant="default" size="xs" onClick={() => logout.mutate({})} loading={logout.isPending}>
				Sign out
			</Button>
		</div>
		{logout.isError && <Alert color="red" title="Could not sign out" mt="sm">{String(logout.error)}</Alert>}
	</div>;
}

/** Pending changes, the last sync, and a button to sync now. */
function SyncRow({ status }: { status: SyncStatus }) {
	const syncNow = useSyncNow();
	const lastSynced = status.lastSyncedAt ? dayjs(status.lastSyncedAt).format("D MMM YYYY, HH:mm") : "Not synced yet";
	const waiting = status.pending === 1 ? "1 change waiting to upload" : `${status.pending} changes waiting to upload`;

	return <Stack gap="xs">
		<Group justify="space-between" wrap="nowrap">
			<Stack gap={2}>
				<Text size="sm">{status.syncing ? "Syncing…" : status.pending > 0 ? waiting : "Everything is synced"}</Text>
				<Text size="xs" c="dimmed">Last synced: {lastSynced}</Text>
			</Stack>
			<Button variant="subtle" size="xs" onClick={() => syncNow.mutate({})} loading={syncNow.isPending} disabled={status.syncing}>
				Sync now
			</Button>
		</Group>
		{status.lastError && <Alert color="red" title="Last sync failed">{status.lastError}</Alert>}
		{syncNow.isError && <Alert color="red" title="Could not sync">{String(syncNow.error)}</Alert>}
	</Stack>;
}

function CreateFamilyCard() {
	const create = useFamilyCreate();
	const form = useForm({
		defaultValues: { name: "" },
		validators: { onChange: familyCreateSchema },
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

function JoinFamilyCard() {
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

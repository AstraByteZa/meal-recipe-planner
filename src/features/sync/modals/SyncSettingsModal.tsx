import { useQuery } from "@tanstack/react-query";
import { modals, type ContextModalProps } from "@mantine/modals";
import { Alert, Button, Group, Stack, Text } from "@mantine/core";
import dayjs from "../../../lib/dayjs";
import { AccountForm } from "../components/AccountForm";
import { FamilyStartCards } from "../components/FamilyStartCards";
import { syncStatusQueryOptions } from "../queries";
import { useSyncLogout, useSyncNow } from "../mutations";
import type { SyncStatus } from "../schema";
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
			: <FamilyStartCards />}
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

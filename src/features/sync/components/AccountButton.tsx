import { IconChevronUp } from "@tabler/icons-react";
import { modals } from "@mantine/modals";
import { Avatar, Menu, Text, UnstyledButton } from "@mantine/core";
import { useQuery } from "@tanstack/react-query";
import { syncStatusQueryOptions } from "../queries";
import { openSyncSettingsModal } from "../modals/SyncSettingsModal";
import { useSyncLogout, useSyncNow, useSyncResetToLocal } from "../mutations";
import classes from "./AccountButton.module.css";

/**
 * Sidebar footer that shows the account. Local or signed out, it opens the sync settings so the
 * user can sign in. Signed in, it opens a menu with sync and sign-out choices.
 */
export function AccountButton() {
	const { data: status } = useQuery(syncStatusQueryOptions);
	const syncNow = useSyncNow();
	const logout = useSyncLogout();
	const resetToLocal = useSyncResetToLocal();

	if (!status) return null;

	if (status.mode !== "signed_in" || !status.user) {
		const local = status.mode === "local";
		return <UnstyledButton className={classes.account} onClick={openSyncSettingsModal}>
			<Avatar radius="xl" color="gray">?</Avatar>
			<span className={classes.text}>
				<span className={classes.name}>{local ? "Local only" : "Signed out"}</span>
				<span className={classes.email}>Not signed in</span>
			</span>
		</UnstyledButton>;
	}

	const { user } = status;
	const actionError = syncNow.error ?? logout.error ?? resetToLocal.error;

	return <>
		<Menu position="top-start" width={260} withArrow>
			<Menu.Target>
				<UnstyledButton className={classes.account} aria-label="Account menu">
					<Avatar radius="xl" color="lime">{initials(user.name)}</Avatar>
					<span className={classes.text}>
						<span className={classes.name}>{user.name}</span>
						<span className={classes.email}>{user.email}</span>
					</span>
					<IconChevronUp size={14} stroke={1.5} />
				</UnstyledButton>
			</Menu.Target>
			<Menu.Dropdown>
				<Menu.Item onClick={() => syncNow.mutate({})} disabled={status.syncing}>Sync now</Menu.Item>
				<Menu.Item onClick={openSyncSettingsModal}>Sync settings</Menu.Item>
				<Menu.Divider />
				<Menu.Item onClick={() => logout.mutate({})}>Sign out</Menu.Item>
				<Menu.Item color="red" onClick={() => confirmResetToLocal(() => resetToLocal.mutate({}))}>
					Sign out and return to local only
				</Menu.Item>
			</Menu.Dropdown>
		</Menu>
		{actionError && <Text size="xs" c="red" mt={4}>{String(actionError)}</Text>}
	</>;
}

function confirmResetToLocal(onConfirm: () => void) {
	modals.openConfirmModal({
		title: "Return this device to local only?",
		children: "Your recipes and plans stay on this device. It leaves the family and stops syncing. The server keeps its copy of the data.",
		labels: { confirm: "Return to local only", cancel: "Cancel" },
		confirmProps: { color: "red" },
		onConfirm,
	});
}

function initials(name: string) {
	const letters = name
		.split(/\s+/)
		.filter(Boolean)
		.slice(0, 2)
		.map((word) => word[0]?.toUpperCase() ?? "")
		.join("");
	return letters || "?";
}

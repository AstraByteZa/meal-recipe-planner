import { useQuery } from "@tanstack/react-query";
import { UnstyledButton } from "@mantine/core";
import { syncStatusQueryOptions } from "../queries";
import { openSyncSettingsModal } from "../modals/SyncSettingsModal";
import classes from "./SyncButton.module.css";

/** Sidebar entry that shows the sync state and opens the sync settings. */
export function SyncButton() {
	const { data: status } = useQuery(syncStatusQueryOptions);

	const { label, state } = !status
		? { label: "Sync", state: "off" }
		: !status.signedIn
			? { label: "Sync off", state: "off" }
			: status.lastError
				? { label: "Sync error", state: "error" }
				: status.syncing || status.pending > 0
					? { label: status.syncing ? "Syncing…" : `${status.pending} waiting`, state: "busy" }
					: { label: "Synced", state: "ok" };

	return <UnstyledButton className={classes.button} onClick={openSyncSettingsModal}>
		<span className={classes.dot} data-state={state} />
		{label}
	</UnstyledButton>;
}

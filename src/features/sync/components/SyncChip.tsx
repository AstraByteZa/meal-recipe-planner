import { useQuery } from "@tanstack/react-query";
import { UnstyledButton } from "@mantine/core";
import { chipState } from "../chip";
import { syncStatusQueryOptions } from "../queries";
import { openSyncSettingsModal } from "../modals/SyncSettingsModal";
import classes from "./SyncChip.module.css";

/** Sidebar chip that shows the sync state. Clicking it opens the sync settings. */
export function SyncChip() {
	const { data: status } = useQuery(syncStatusQueryOptions);
	if (!status) return null;

	const { state, label, detail } = chipState(status, new Date());

	return <UnstyledButton className={classes.chip} onClick={openSyncSettingsModal} aria-label={`${label}. ${detail}`}>
		<span className={classes.dot} data-state={state} />
		<span className={classes.text}>
			<span className={classes.label}>{label}</span>
			<span className={classes.detail}>{detail}</span>
		</span>
	</UnstyledButton>;
}

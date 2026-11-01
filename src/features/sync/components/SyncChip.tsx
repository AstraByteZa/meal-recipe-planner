import { useQuery } from "@tanstack/react-query";
import { ActionIcon, UnstyledButton } from "@mantine/core";
import { IconCloud, IconCloudOff } from "@tabler/icons-react";
import { chipState } from "../chip";
import { syncStatusQueryOptions } from "../queries";
import { openSyncSettingsModal } from "../modals/SyncSettingsModal";
import classes from "./SyncChip.module.css";

interface SyncChipProps {
	/** Icon only, for the narrow-screen header. */
	compact?: boolean;
}

/** Shows the sync state. Clicking it opens the sync settings. */
export function SyncChip({ compact = false }: SyncChipProps) {
	const { data: status } = useQuery(syncStatusQueryOptions);
	if (!status) return null;

	const { state, label, detail } = chipState(status, new Date());

	if (compact) {
		const Icon = status.mode === "signed_in" ? IconCloud : IconCloudOff;
		return <ActionIcon variant="default" size="lg" onClick={openSyncSettingsModal} aria-label={`${label}. ${detail}`} className={classes.compact}>
			<Icon size={18} stroke={1.5} />
			<span className={classes.compactDot} data-state={state} />
		</ActionIcon>;
	}

	return <UnstyledButton className={classes.chip} onClick={openSyncSettingsModal} aria-label={`${label}. ${detail}`}>
		<span className={classes.dot} data-state={state} />
		<span className={classes.text}>
			<span className={classes.label}>{label}</span>
			<span className={classes.detail}>{detail}</span>
		</span>
	</UnstyledButton>;
}

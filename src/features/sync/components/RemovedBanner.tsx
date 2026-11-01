import { IconUserMinus, IconX } from "@tabler/icons-react";
import { useQuery } from "@tanstack/react-query";
import { ActionIcon, Text } from "@mantine/core";
import { noticeText } from "../format";
import { syncStatusQueryOptions } from "../queries";
import { useSyncDismissNotice } from "../mutations";
import type { SyncStatus } from "../schema";
import classes from "./RemovedBanner.module.css";

/**
 * Shown after this device was removed from a family, or its family was deleted. It stays until the
 * user dismisses it, so the change is never missed.
 */
export function RemovedBanner({ notice }: { notice: NonNullable<SyncStatus["notice"]> }) {
	const dismiss = useSyncDismissNotice();

	return <div className={classes.banner} role="status">
		<IconUserMinus size={18} stroke={1.5} />
		<Text className={classes.text}>{noticeText(notice.kind, notice.familyName)}</Text>
		<ActionIcon
			variant="subtle"
			color="gray"
			aria-label="Dismiss"
			loading={dismiss.isPending}
			onClick={() => dismiss.mutate({})}
		>
			<IconX size={14} stroke={1.5} />
		</ActionIcon>
	</div>;
}

/** Reads the status and shows the removed or deleted banner when there is one. */
export function RemovedNotice() {
	const { data: status } = useQuery(syncStatusQueryOptions);
	if (!status?.notice) return null;
	return <RemovedBanner notice={status.notice} />;
}

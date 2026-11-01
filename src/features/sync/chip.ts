import type { SyncStatus } from "./schema";
import { relativeTime } from "./format";

export type ChipState = "off" | "error" | "busy" | "waiting" | "ok";

export interface Chip {
	state: ChipState;
	label: string;
	detail: string;
}

/**
 * The sidebar sync chip. Covers every account state: local only and signed out show as off,
 * and a signed-in device shows the first of: unauthorized, failed, syncing, waiting, synced.
 */
export function chipState(status: SyncStatus, now: Date): Chip {
	if (status.mode !== "signed_in") {
		return { state: "off", label: "Sync off", detail: "Sign in to sync" };
	}
	if (status.lastErrorKind === "unauthorized") {
		return { state: "error", label: "Sign in again", detail: "Your session has ended" };
	}
	if (status.lastError) {
		const detail = status.lastErrorKind === "unreachable" ? "Server unreachable. Retry" : "Last sync failed";
		return { state: "error", label: "Sync failed", detail };
	}
	if (status.syncing) {
		const detail = status.pending > 0 ? `Sending ${status.pending} ${plural(status.pending)}` : "Checking for changes";
		return { state: "busy", label: "Syncing…", detail };
	}
	if (status.pending > 0) {
		return {
			state: "waiting",
			label: `${status.pending} ${plural(status.pending)} waiting`,
			detail: "Will sync when online",
		};
	}
	const detail = status.lastSyncedAt ? `Last synced ${relativeTime(status.lastSyncedAt, now)}` : "Not synced yet";
	return { state: "ok", label: "Synced", detail };
}

function plural(count: number) {
	return count === 1 ? "change" : "changes";
}

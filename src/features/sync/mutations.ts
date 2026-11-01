import { useMutation, useQueryClient } from "@tanstack/react-query";
import { invoke } from "@tauri-apps/api/core";
import { syncStatusQueryKey } from "./queries";
import { invitePreviewSchema, syncStatusSchema } from "./schema";

/**
 * Runs a sync command and stores the status it returns. Errors are not caught here: the Rust
 * side returns a message string, and the caller shows it.
 */
function useSyncCommand<TArgs extends Record<string, unknown>>(command: string) {
	const queryClient = useQueryClient();

	return useMutation({
		mutationFn: async (args: TArgs) =>
			syncStatusSchema.parse(await invoke<unknown>(command, args)),
		onSuccess: (status) => queryClient.setQueryData(syncStatusQueryKey, status),
	});
}

export function useSyncRegister() {
	return useSyncCommand<{ serverUrl: string; email: string; password: string; displayName?: string }>(
		"sync_register",
	);
}

/** Changes the signed-in user's display name on the server and on this device. */
export function useSyncSetName() {
	return useSyncCommand<{ name: string }>("sync_set_name");
}

/**
 * Makes this device local again: rows stay, leave the family and stop syncing. Pass `{}`.
 * The server keeps its copy of the data.
 */
export function useSyncResetToLocal() {
	return useSyncCommand<Record<string, never>>("sync_reset_to_local");
}

export function useSyncLogin() {
	return useSyncCommand<{ serverUrl: string; email: string; password: string }>("sync_login");
}

/** Pass `{}`. The command takes no arguments. */
export function useSyncLogout() {
	return useSyncCommand<Record<string, never>>("sync_logout");
}

/** Pass `{}`. The command takes no arguments. */
export function useSyncNow() {
	return useSyncCommand<Record<string, never>>("sync_now");
}

export function useFamilyCreate() {
	return useSyncCommand<{ name: string }>("family_create");
}

export function useFamilyJoin() {
	return useSyncCommand<{ inviteCode: string }>("family_join");
}

/**
 * Looks up an invite code without using it. The result is not stored in the status cache: the
 * caller shows it and decides whether to accept.
 */
export function useFamilyPreview() {
	return useMutation({
		mutationFn: async (args: { inviteCode: string }) =>
			invitePreviewSchema.parse(await invoke<unknown>("family_invite_preview", args)),
	});
}

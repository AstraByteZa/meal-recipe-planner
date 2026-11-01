import { useMutation, useQueryClient } from "@tanstack/react-query";
import { invoke } from "@tauri-apps/api/core";
import { syncStatusQueryKey } from "./queries";
import { familyInviteSchema, invitePreviewSchema, syncStatusSchema } from "./schema";

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

/**
 * Like `useSyncCommand`, and also refreshes the family details. Use it for family writes that
 * return a status: the members, the invites and the family name can all have changed.
 */
function useFamilyStatusCommand<TArgs extends Record<string, unknown>>(command: string) {
	const queryClient = useQueryClient();

	return useMutation({
		mutationFn: async (args: TArgs) =>
			syncStatusSchema.parse(await invoke<unknown>(command, args)),
		onSuccess: (status) => {
			queryClient.setQueryData(syncStatusQueryKey, status);
			void queryClient.invalidateQueries({ queryKey: ["sync", "family"] });
		},
	});
}

/** Refreshes the family details after a write that does not return a status. */
function useFamilyDetailsRefresh() {
	const queryClient = useQueryClient();
	return () => queryClient.invalidateQueries({ queryKey: ["sync", "family"] });
}

export function useSyncRegister() {
	return useSyncCommand<{ serverUrl?: string; email: string; password: string; displayName?: string }>(
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
	return useSyncCommand<{ serverUrl?: string; email: string; password: string }>("sync_login");
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

/** Renames the family. Owner only. */
export function useFamilyRename() {
	return useFamilyStatusCommand<{ name: string }>("family_rename");
}

/** Removes a member, or leaves when `userId` is the caller. The owner cannot be removed. */
export function useFamilyMemberRemove() {
	return useFamilyStatusCommand<{ userId: string }>("family_member_remove");
}

/** Makes another member the owner. The caller becomes a regular member. */
export function useFamilyTransfer() {
	return useFamilyStatusCommand<{ userId: string }>("family_transfer");
}

/** Deletes the family for everyone. Pass `{}`. Owner only. */
export function useFamilyDelete() {
	return useFamilyStatusCommand<Record<string, never>>("family_delete");
}

/** Clears the removed or deleted notice. Pass `{}`. */
export function useSyncDismissNotice() {
	return useSyncCommand<Record<string, never>>("sync_dismiss_notice");
}

/** Creates a single-use invite code. The owner only. */
export function useFamilyInviteCreate() {
	const refresh = useFamilyDetailsRefresh();

	return useMutation({
		mutationFn: async () => familyInviteSchema.parse(await invoke<unknown>("family_invite_create")),
		onSuccess: () => refresh(),
	});
}

/** Revokes an invite that has not been used yet. The owner only. */
export function useFamilyInviteRevoke() {
	const refresh = useFamilyDetailsRefresh();

	return useMutation({
		mutationFn: async (args: { inviteId: string }) => {
			await invoke<void>("family_invite_revoke", args);
		},
		onSuccess: () => refresh(),
	});
}

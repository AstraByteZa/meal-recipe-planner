import { useMutation, useQueryClient } from "@tanstack/react-query";
import { invoke } from "@tauri-apps/api/core";
import { syncStatusQueryKey } from "./queries";
import { syncStatusSchema } from "./schema";

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
	return useSyncCommand<{ serverUrl: string; email: string; password: string }>("sync_register");
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

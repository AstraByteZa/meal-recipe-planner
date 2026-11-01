import { queryOptions } from "@tanstack/react-query";
import { invoke } from "@tauri-apps/api/core";
import { familyDetailsSchema, syncStatusSchema } from "./schema";

export const syncStatusQueryKey = ["sync"] as const;

export const syncStatusQueryOptions = queryOptions({
	queryKey: syncStatusQueryKey,
	queryFn: async () => syncStatusSchema.parse(await invoke<unknown>("sync_status")),
	// The worker pushes and pulls in the background, which changes the pending count and last
	// sync time without a command result to update the cache.
	refetchInterval: 10_000,
});

/**
 * The Family page's data: members, and the invites when the caller owns the family. Keyed by
 * family id, so a stale family never shows. Every family write invalidates `["sync", "family"]`.
 */
export const familyDetailsQueryKey = (familyId: string) => ["sync", "family", familyId] as const;

export const familyDetailsQueryOptions = (familyId: string) =>
	queryOptions({
		queryKey: familyDetailsQueryKey(familyId),
		queryFn: async () => familyDetailsSchema.parse(await invoke<unknown>("family_details")),
	});

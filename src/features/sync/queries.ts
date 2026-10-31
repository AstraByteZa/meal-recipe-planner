import { queryOptions } from "@tanstack/react-query";
import { invoke } from "@tauri-apps/api/core";
import { syncStatusSchema } from "./schema";

export const syncStatusQueryKey = ["sync"] as const;

export const syncStatusQueryOptions = queryOptions({
	queryKey: syncStatusQueryKey,
	queryFn: async () => syncStatusSchema.parse(await invoke<unknown>("sync_status")),
	// The worker pushes and pulls in the background, which changes the pending count and last
	// sync time without a command result to update the cache.
	refetchInterval: 10_000,
});

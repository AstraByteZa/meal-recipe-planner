import { queryOptions } from "@tanstack/react-query";
import { invoke } from "@tauri-apps/api/core";
import type { Settings } from "./types";

export const settingsQueryOptions = queryOptions({
	queryKey: ["settings"],
	queryFn: () => invoke<Settings>("settings_get"),
});

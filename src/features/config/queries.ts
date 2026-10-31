import { queryOptions } from "@tanstack/react-query";
import { invoke } from "@tauri-apps/api/core";
import { appConfigSchema } from "./schema";

export const configQueryOptions = queryOptions({
	queryKey: ["config"],
	queryFn: async () => appConfigSchema.parse(await invoke<unknown>("config_list")),
});

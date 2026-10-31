import { useMutation, useQueryClient } from "@tanstack/react-query";
import { invoke } from "@tauri-apps/api/core";
import { notifications } from "@mantine/notifications";
import type { AppConfig } from "./schema";

/** One setting and its new value. The value type follows the key. */
export type ConfigChange = {
	[K in keyof AppConfig]: { key: K; value: AppConfig[K] };
}[keyof AppConfig];

export function useSetConfig() {
	const queryClient = useQueryClient();

	return useMutation({
		mutationFn: ({ key, value }: ConfigChange) => invoke<void>("config_set", { key, value }),
		onSuccess: () => queryClient.invalidateQueries({ queryKey: ["config"] }),
		// The switch and buttons show the saved value, so a failed save would otherwise look like a missed click.
		onError: (error) => notifications.show({
			color: "red",
			title: "Could not save the setting",
			message: String(error),
			autoClose: 8000,
		}),
	});
}

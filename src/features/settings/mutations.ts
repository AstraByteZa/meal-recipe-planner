import { useMutation, useQueryClient } from "@tanstack/react-query";
import { invoke } from "@tauri-apps/api/core";
import type { SettingsInput } from "./types";

export function useSaveSettings() {
	const queryClient = useQueryClient();

	return useMutation({
		mutationFn: (input: SettingsInput) => invoke<void>("settings_save", { input }),
		onSuccess: () => queryClient.invalidateQueries({ queryKey: ["settings"] }),
	});
}

/** Marks onboarding as finished. The app layout stops redirecting after this. */
export function useCompleteOnboarding() {
	const queryClient = useQueryClient();

	return useMutation({
		mutationFn: () => invoke<void>("settings_complete_onboarding"),
		onSuccess: () => queryClient.invalidateQueries({ queryKey: ["settings"] }),
	});
}

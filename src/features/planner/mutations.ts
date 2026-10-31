import { useMutation, useQueryClient } from "@tanstack/react-query";
import { invoke } from "@tauri-apps/api/core";
import type { Track } from "./types";

export interface SetScheduleEntryInput {
	date: string;
	trackKey: string;
	recipeId: string;
}

export function useSetScheduleEntry() {
	const queryClient = useQueryClient();

	return useMutation({
		mutationFn: ({ date, trackKey, recipeId }: SetScheduleEntryInput) =>
			invoke<void>("schedule_set_entry", { date, trackKey, recipeId }),
		// Refetches every schedule range so the grid and past weeks show the new meal.
		// Recipe usage counts and plan history change too.
		onSuccess: () => Promise.all([
			queryClient.invalidateQueries({ queryKey: ["schedule"] }),
			queryClient.invalidateQueries({ queryKey: ["recipes"] }),
		]),
	});
}

export interface ClearScheduleEntryInput {
	date: string;
	trackKey: string;
}

export function useClearScheduleEntry() {
	const queryClient = useQueryClient();

	return useMutation({
		mutationFn: ({ date, trackKey }: ClearScheduleEntryInput) =>
			invoke<void>("schedule_clear_entry", { date, trackKey }),
		onSuccess: () => Promise.all([
			queryClient.invalidateQueries({ queryKey: ["schedule"] }),
			queryClient.invalidateQueries({ queryKey: ["recipes"] }),
		]),
	});
}

/** Replaces the track list. The array order is the schedule order. */
export function useSaveTracks() {
	const queryClient = useQueryClient();

	return useMutation({
		mutationFn: (tracks: Omit<Track, "position">[]) => invoke<void>("tracks_save", { tracks }),
		// Removed tracks take their planned meals with them, so the schedule refetches too.
		onSuccess: () => Promise.all([
			queryClient.invalidateQueries({ queryKey: ["tracks"] }),
			queryClient.invalidateQueries({ queryKey: ["schedule"] }),
		]),
	});
}

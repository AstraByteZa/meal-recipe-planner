import { useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { notifications } from "@mantine/notifications";

/**
 * Keeps the screen on while cook mode is open and the setting is on. Leaving cook mode releases it.
 * The Rust side holds the OS inhibitor: navigator.wakeLock is refused by WebKitGTK on Linux.
 */
export function useKeepAwake(enabled: boolean) {
	useEffect(() => {
		invoke<void>("keep_awake_set", { enabled }).catch((error: unknown) => {
			notifications.show({
				color: "red",
				title: "Could not keep the screen awake",
				message: String(error),
				autoClose: 8000,
			});
		});
	}, [enabled]);

	useEffect(() => () => {
		void invoke<void>("keep_awake_set", { enabled: false });
	}, []);
}

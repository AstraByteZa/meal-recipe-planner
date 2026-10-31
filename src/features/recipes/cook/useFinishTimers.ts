import { useEffect } from "react";
import { useAtomValue, useStore } from "jotai";
import { notifications } from "@mantine/notifications";
import { appConfigAtom } from "../../config/atoms";
import { timersAtom } from "./atoms";
import { playChime } from "./sound";
import { finishTimer, isDue } from "./timers";

/** Checks the running timers once a second. A timer that has reached its end shows a notification and, if the setting is on, a chime. */
export function useFinishTimers() {
	const store = useStore();
	const soundOn = useAtomValue(appConfigAtom)["cook-timer-sound"];

	useEffect(() => {
		const check = () => {
			const now = Date.now();
			const timers = store.get(timersAtom);
			const due = Object.values(timers).filter((timer) => isDue(timer, now));
			if (due.length === 0) return;

			store.set(timersAtom, {
				...timers,
				...Object.fromEntries(due.map((timer) => [timer.id, finishTimer(timer)])),
			});
			for (const timer of due) {
				notifications.show({
					title: "Timer finished",
					message: `${timer.label}, step ${timer.step}`,
					autoClose: 8000,
				});
			}
			if (soundOn) playChime();
		};

		const id = setInterval(check, 1000);
		return () => clearInterval(id);
	}, [store, soundOn]);
}

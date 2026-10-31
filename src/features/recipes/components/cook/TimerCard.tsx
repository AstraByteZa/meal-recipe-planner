import { useEffect, useReducer, useState } from "react";
import { Button, NumberInput, Text } from "@mantine/core";
import { formatRemaining, MAX_MINUTES, MINUTE_MS, remainingAt, type CookTimer } from "../../cook/timers";
import classes from "./TimerCard.module.css";

interface TimerCardProps {
	timer: CookTimer;
	onStart: () => void;
	onPause: () => void;
	onAddMinute: () => void;
	onReset: () => void;
	/** Sets a new length. Only called while the timer is idle. */
	onMinutesChange: (minutes: number) => void;
}

/** Re-renders once a second while the timer runs, so the countdown stays current. */
function useTicker(active: boolean) {
	const [, tick] = useReducer((count: number) => count + 1, 0);

	useEffect(() => {
		if (!active) return;
		const id = setInterval(tick, 1000);
		return () => clearInterval(id);
	}, [active]);
}

export function TimerCard({ timer, onStart, onPause, onAddMinute, onReset, onMinutesChange }: TimerCardProps) {
	useTicker(timer.status === "running");

	const remaining = remainingAt(timer, Date.now());
	const progress = 1 - remaining / timer.totalMs;
	const finished = timer.status === "done";
	const editable = timer.status === "idle";
	const minutes = timer.totalMs / MINUTE_MS;

	// The field keeps what the user is typing, so clearing it is possible. A valid number is saved as it changes.
	const [draft, setDraft] = useState<number | string>(minutes);
	useEffect(() => setDraft(minutes), [minutes]);

	return <section className={classes.card}>
		<div className={classes.time}>
			<Text className={classes.label}>
				{finished ? `${timer.label} · finished` : editable ? `${timer.label} · minutes` : timer.label}
			</Text>
			{editable
				? <NumberInput
					className={classes.minutesField}
					classNames={{ input: classes.minutesInput }}
					aria-label="Timer minutes"
					value={draft}
					min={1}
					max={MAX_MINUTES}
					allowDecimal={false}
					allowNegative={false}
					hideControls
					variant="unstyled"
					onChange={(value) => {
						setDraft(value);
						if (typeof value === "number" && value >= 1) onMinutesChange(value);
					}}
					onBlur={() => setDraft(minutes)}
				/>
				: <Text className={classes.display}>{formatRemaining(remaining)}</Text>}
		</div>

		<div className={classes.side}>
			<div className={classes.track}>
				<span className={classes.fill} style={{ width: `${progress * 100}%` }} />
			</div>
			<div className={classes.buttons}>
				{timer.status === "running"
					? <Button variant="default" onClick={onPause}>Pause</Button>
					: <Button onClick={onStart}>{timer.status === "paused" ? "Resume" : "Start"}</Button>}
				<Button variant="default" onClick={onAddMinute}>+1 min</Button>
				<Button variant="default" onClick={onReset}>Reset</Button>
			</div>
		</div>
	</section>;
}

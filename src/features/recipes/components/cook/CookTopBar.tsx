import { IconX } from "@tabler/icons-react";
import { Button, Group, Switch, Text, Title } from "@mantine/core";
import { TEXT_SCALES, type TextScale } from "../../cook/textScale";
import classes from "./CookTopBar.module.css";

interface CookTopBarProps {
	recipeName: string;
	current: number;
	done: number[];
	total: number;
	minutesLeft: number;
	keepAwake: boolean;
	textScale: TextScale;
	onExit: () => void;
	onKeepAwakeChange: (keepAwake: boolean) => void;
	onTextScaleChange: (textScale: TextScale) => void;
}

export function CookTopBar({
	recipeName,
	current,
	done,
	total,
	minutesLeft,
	keepAwake,
	textScale,
	onExit,
	onKeepAwakeChange,
	onTextScaleChange,
}: CookTopBarProps) {
	const scaleIndex = TEXT_SCALES.indexOf(textScale);

	return <header className={classes.bar}>
		<Button variant="default" leftSection={<IconX size={16} stroke={1.5} />} onClick={onExit}>
			Exit cook mode
		</Button>

		<div className={classes.title}>
			<Text className={classes.label}>Cooking now</Text>
			<Title order={3}>{recipeName}</Title>
		</div>

		<div className={classes.progress}>
			<div className={classes.progressText}>
				<Text size="sm" c="dimmed">Step {current + 1} of {total}</Text>
				<Text size="sm" c="dimmed">{minutesLeft > 0 ? `About ${minutesLeft} min left` : "No timers left"}</Text>
			</div>
			<div className={classes.segments}>
				{Array.from({ length: total }, (_, index) => {
					const state = done.includes(index) ? "done" : index === current ? "current" : "upcoming";
					return <span key={index} className={`${classes.segment} ${classes[state]}`} />;
				})}
			</div>
		</div>

		<Group gap="md" wrap="nowrap">
			<Switch
				label="Keep awake"
				checked={keepAwake}
				onChange={(event) => onKeepAwakeChange(event.currentTarget.checked)}
			/>
			<Group gap="xs" wrap="nowrap">
				<Button
					variant="default"
					size="compact-md"
					aria-label="Smaller text"
					disabled={scaleIndex === 0}
					onClick={() => onTextScaleChange(TEXT_SCALES[scaleIndex - 1])}
				>
					A-
				</Button>
				<Button
					variant="default"
					size="compact-xl"
					aria-label="Larger text"
					disabled={scaleIndex === TEXT_SCALES.length - 1}
					onClick={() => onTextScaleChange(TEXT_SCALES[scaleIndex + 1])}
				>
					A+
				</Button>
			</Group>
		</Group>
	</header>;
}

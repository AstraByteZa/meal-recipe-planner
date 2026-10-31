import { Text, Title } from "@mantine/core";
import { STEP_COUNT } from "../steps";
import classes from "./StepIntro.module.css";

interface StepIntroProps {
	step: number;
	title: string;
	description: string;
}

export function StepIntro({ step, title, description }: StepIntroProps) {
	return <header className={classes.intro}>
		<Text size="xs" c="dimmed" className={classes.eyebrow}>Step {step} of {STEP_COUNT}</Text>
		<Title order={1} className={classes.title}>{title}</Title>
		<Text c="dimmed" className={classes.description}>{description}</Text>
	</header>
}

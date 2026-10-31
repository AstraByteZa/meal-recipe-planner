import { Text, Title } from "@mantine/core";
import classes from "./StepPanel.module.css";

interface StepPanelProps {
	index: number;
	total: number;
	text: string;
}

export function StepPanel({ index, total, text }: StepPanelProps) {
	return <section className={classes.panel}>
		<Text className={classes.label}>Step {index + 1} of {total}</Text>
		<Title order={2} className={classes.text}>{text}</Title>
	</section>;
}

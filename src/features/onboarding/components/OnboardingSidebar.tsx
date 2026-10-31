import { IconCheck, IconCircleDot } from "@tabler/icons-react";
import { Box, Text, UnstyledButton } from "@mantine/core";
import { STEPS } from "../steps";
import classes from "./OnboardingSidebar.module.css";

interface OnboardingSidebarProps {
	current: number;
	onSelect: (step: number) => void;
}

/** Step list. Finished steps can be revisited. Later steps cannot be skipped to. */
export function OnboardingSidebar({ current, onSelect }: OnboardingSidebarProps) {
	return <nav className={classes.sidebar} aria-label="Onboarding steps">
		<div className={classes.brand}>
			<Box className={classes.logo}>
				<IconCircleDot size={16} stroke={2} />
			</Box>
			<Text fw={700} size="lg">Mise</Text>
		</div>

		<ol className={classes.steps}>
			{STEPS.map((step, index) => {
				const number = index + 1;
				const isDone = number < current;
				const isCurrent = number === current;

				const content = <>
					<span className={classes.marker}>
						{isDone ? <IconCheck size={14} stroke={2.5} /> : number}
					</span>
					<span className={classes.text}>
						<span className={classes.title}>{step.title}</span>
						<span className={classes.subtitle}>{step.subtitle}</span>
					</span>
				</>;

				const className = [
					classes.step,
					isDone && classes.stepDone,
					isCurrent && classes.stepCurrent,
				].filter(Boolean).join(" ");

				return <li key={step.title}>
					{isDone
						? <UnstyledButton className={className} onClick={() => onSelect(number)}>{content}</UnstyledButton>
						: <div className={className} aria-current={isCurrent ? "step" : undefined}>{content}</div>}
				</li>
			})}
		</ol>

		<Text size="xs" c="dimmed" className={classes.footnote}>About 2 minutes. You can change everything later.</Text>
	</nav>
}

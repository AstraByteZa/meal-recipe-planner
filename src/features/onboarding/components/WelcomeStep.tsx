import { IconAdjustmentsHorizontal, IconCalendarWeek, IconLink } from "@tabler/icons-react";
import { Stack, Text } from "@mantine/core";
import { StepFooter } from "./StepFooter";
import { StepIntro } from "./StepIntro";
import classes from "./WelcomeStep.module.css";

const features = [
	{ icon: IconCalendarWeek, title: "Plan the week", text: "Drop meals into your tracks and look back at how past weeks went." },
	{ icon: IconLink, title: "Keep your recipe links", text: "Save links, rate them and see when you last cooked each one." },
	{ icon: IconAdjustmentsHorizontal, title: "Shape your schedule", text: "Add, rename or hide tracks to match how you actually eat." },
];

interface WelcomeStepProps {
	onNext: () => void;
}

export function WelcomeStep({ onNext }: WelcomeStepProps) {
	return <>
		<StepIntro
			step={1}
			title="Plan meals around the recipes you already love"
			description="Mise keeps your week and your saved recipe links in one calm place."
		/>

		<Stack gap="sm">
			{features.map((feature) => <div key={feature.title} className={classes.feature}>
				<div className={classes.icon}>
					<feature.icon size={18} stroke={1.5} />
				</div>
				<div className={classes.text}>
					<Text fw={600}>{feature.title}</Text>
					<Text size="sm" c="dimmed">{feature.text}</Text>
				</div>
			</div>)}
		</Stack>

		<StepFooter onNext={onNext} nextLabel="Get started" />
	</>
}

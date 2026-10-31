import { IconMinus, IconPlus } from "@tabler/icons-react";
import { useForm } from "@tanstack/react-form";
import { useSuspenseQuery } from "@tanstack/react-query";
import { Text, UnstyledButton } from "@mantine/core";
import { z } from "zod";
import { DIETARY_NEEDS, WEEK_START_OPTIONS } from "../../settings/options";
import { settingsQueryOptions } from "../../settings/queries";
import { useSaveSettings } from "../../settings/mutations";
import type { WeekStart } from "../../settings/types";
import { StepFooter } from "./StepFooter";
import { StepIntro } from "./StepIntro";
import classes from "./HouseholdStep.module.css";

const MIN_PEOPLE = 1;
const MAX_PEOPLE = 99;

const householdSchema = z.object({
	householdSize: z.number().int().min(MIN_PEOPLE).max(MAX_PEOPLE),
	dietary: z.array(z.string()),
	weekStartsOn: z.enum(["monday", "sunday"]),
});

interface HouseholdStepProps {
	onBack: () => void;
	onNext: () => void;
	onSkip: () => void;
}

export function HouseholdStep({ onBack, onNext, onSkip }: HouseholdStepProps) {
	const { data: settings } = useSuspenseQuery(settingsQueryOptions);
	const saveSettings = useSaveSettings();

	const form = useForm({
		defaultValues: {
			householdSize: settings.householdSize,
			dietary: settings.dietary,
			weekStartsOn: settings.weekStartsOn as WeekStart,
		},
		validators: { onChange: householdSchema },
		onSubmit: ({ value }) => saveSettings.mutate(value, { onSuccess: onNext }),
	});

	return <form onSubmit={(event) => {
		event.preventDefault();
		form.handleSubmit();
	}}>
		<div className={classes.sections}>
			<StepIntro
				step={3}
				title="Who are you cooking for?"
				description="This sets sensible defaults and filters what we suggest."
			/>

			<section className={classes.section}>
				<Text size="xs" c="dimmed" className={classes.label}>Cooking for</Text>
				<form.Field name="householdSize">
					{(field) => <div className={classes.stepper}>
						<UnstyledButton
							className={classes.stepButton}
							aria-label="One person fewer"
							disabled={field.state.value <= MIN_PEOPLE}
							onClick={() => field.handleChange(Math.max(MIN_PEOPLE, field.state.value - 1))}
						>
							<IconMinus size={16} stroke={1.5} />
						</UnstyledButton>
						<Text className={classes.count} aria-live="polite">{field.state.value}</Text>
						<UnstyledButton
							className={classes.stepButton}
							aria-label="One person more"
							disabled={field.state.value >= MAX_PEOPLE}
							onClick={() => field.handleChange(Math.min(MAX_PEOPLE, field.state.value + 1))}
						>
							<IconPlus size={16} stroke={1.5} />
						</UnstyledButton>
						<Text c="dimmed">people</Text>
					</div>}
				</form.Field>
			</section>

			<section className={classes.section}>
				<Text size="xs" c="dimmed" className={classes.label}>Dietary needs</Text>
				<form.Field name="dietary">
					{(field) => <div className={classes.chips}>
						<UnstyledButton
							className={field.state.value.length === 0 ? `${classes.chip} ${classes.chipActive}` : classes.chip}
							aria-pressed={field.state.value.length === 0}
							onClick={() => field.handleChange([])}
						>
							None
						</UnstyledButton>
						{DIETARY_NEEDS.map((diet) => {
							const isActive = field.state.value.includes(diet.value);

							return <UnstyledButton
								key={diet.value}
								className={isActive ? `${classes.chip} ${classes.chipActive}` : classes.chip}
								aria-pressed={isActive}
								onClick={() => field.handleChange(isActive
									? field.state.value.filter((value) => value !== diet.value)
									: [...field.state.value, diet.value])}
							>
								{diet.label}
							</UnstyledButton>
						})}
					</div>}
				</form.Field>
			</section>

			<section className={classes.section}>
				<Text size="xs" c="dimmed" className={classes.label}>Week starts on</Text>
				<form.Field name="weekStartsOn">
					{(field) => <div className={classes.chips}>
						{WEEK_START_OPTIONS.map((option) => {
							const isActive = field.state.value === option.value;

							return <UnstyledButton
								key={option.value}
								className={isActive ? `${classes.chip} ${classes.chipActive}` : classes.chip}
								aria-pressed={isActive}
								onClick={() => field.handleChange(option.value)}
							>
								{option.label}
							</UnstyledButton>
						})}
					</div>}
				</form.Field>
			</section>
		</div>

		<StepFooter
			onBack={onBack}
			onSkip={onSkip}
			onNext={() => form.handleSubmit()}
			nextLoading={saveSettings.isPending}
		/>
	</form>
}

import { useNavigate } from "@tanstack/react-router";
import { currentWeek } from "../../planner/week";
import { useCompleteOnboarding } from "../../settings/mutations";
import { OnboardingSidebar } from "../components/OnboardingSidebar";
import { FirstPlanStep } from "../components/FirstPlanStep";
import { FirstRecipeStep } from "../components/FirstRecipeStep";
import { HouseholdStep } from "../components/HouseholdStep";
import { TracksStep } from "../components/TracksStep";
import { WelcomeStep } from "../components/WelcomeStep";
import classes from "./OnboardingView.module.css";

interface OnboardingViewProps {
	step: number;
}

export function OnboardingView({ step }: OnboardingViewProps) {
	const navigate = useNavigate();
	const completeOnboarding = useCompleteOnboarding();

	const goTo = (next: number) => navigate({ to: "/onboarding", search: { step: next } });
	const finish = () => completeOnboarding.mutate(undefined, {
		onSuccess: () => navigate({ to: "/", search: { week: currentWeek() }, replace: true }),
	});

	return <div className={classes.view}>
		<OnboardingSidebar current={step} onSelect={goTo} />

		<main className={classes.main}>
			<div className={classes.column}>
				{step === 1 && <WelcomeStep onNext={() => goTo(2)} />}
				{step === 2 && <TracksStep onBack={() => goTo(1)} onSkip={() => goTo(3)} onNext={() => goTo(3)} />}
				{step === 3 && <HouseholdStep onBack={() => goTo(2)} onSkip={() => goTo(4)} onNext={() => goTo(4)} />}
				{step === 4 && <FirstRecipeStep onBack={() => goTo(3)} onSkip={() => goTo(5)} onNext={() => goTo(5)} />}
				{step === 5 && <FirstPlanStep onBack={() => goTo(4)} onFinish={finish} finishing={completeOnboarding.isPending} />}
			</div>
		</main>
	</div>
}

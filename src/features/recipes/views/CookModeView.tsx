import { useEffect, type CSSProperties } from "react";
import { useSuspenseQuery } from "@tanstack/react-query";
import { useNavigate } from "@tanstack/react-router";
import { useAtom, useAtomValue } from "jotai";
import { Alert, Button } from "@mantine/core";
import { IconChevronLeft } from "@tabler/icons-react";
import { appConfigAtom } from "../../config/atoms";
import { useSetConfig } from "../../config/mutations";
import { CookNav } from "../components/cook/CookNav";
import { CookTopBar } from "../components/cook/CookTopBar";
import { IngredientChecklist } from "../components/cook/IngredientChecklist";
import { StepList } from "../components/cook/StepList";
import { StepPanel } from "../components/cook/StepPanel";
import { checkedIngredientsAtom, doneStepsAtom, timersAtom } from "../cook/atoms";
import { timeLeft } from "../cook/steps";
import { TEXT_SCALE_FACTORS } from "../cook/textScale";
import { useFinishTimers } from "../cook/useFinishTimers";
import { useKeepAwake } from "../cook/useKeepAwake";
import { addMinute, newTimer, pauseTimer, resetTimer, setTimerMinutes, startTimer, timersForStep, type CookTimer } from "../cook/timers";
import { recipeQueryOptions } from "../queries";
import { useUpdateRecipe } from "../mutations";
import { recipeToInput } from "../recipeInput";
import { TimerCard } from "../components/cook/TimerCard";
import { CookDone, type CookDoneValues } from "../components/cook/CookDone";
import classes from "./CookModeView.module.css";

interface CookModeViewProps {
	recipeId: string;
	/** A zero-based step index, or "done" for the screen after the last step. */
	step: number | "done";
}

export function CookModeView({ recipeId, step }: CookModeViewProps) {
	const { data: recipe } = useSuspenseQuery(recipeQueryOptions(recipeId));
	const navigate = useNavigate();
	const updateRecipe = useUpdateRecipe(recipe.id);
	const [checked, setChecked] = useAtom(checkedIngredientsAtom);
	const [done, setDone] = useAtom(doneStepsAtom);
	const [timers, setTimers] = useAtom(timersAtom);
	const config = useAtomValue(appConfigAtom);
	const setConfig = useSetConfig();
	useFinishTimers();
	useKeepAwake(config["cook-keep-awake"]);

	const total = recipe.instructions.length;
	const lastStep = total - 1;
	const isDone = step === "done";
	// A step number past the end (typed into the URL) shows the last step.
	const current = isDone ? lastStep : Math.max(0, Math.min(step, lastStep));

	// Each visit starts a fresh cook session.
	useEffect(() => {
		setChecked([]);
		setDone([]);
		setTimers({});
	}, [setChecked, setDone, setTimers]);

	const goTo = (index: number | "done") => navigate({
		to: "/recipes/$recipeId/cook",
		params: { recipeId: recipe.id },
		search: { step: index },
	});
	const exit = () => navigate({ to: "/recipes/$recipeId", params: { recipeId: recipe.id } });
	// Next on the last step opens the done screen.
	const next = () => {
		setDone((previous) => (previous.includes(current) ? previous : [...previous, current]));
		void goTo(current === lastStep ? "done" : current + 1);
	};
	// Saves the rating and notes only if they changed, then returns to the recipe.
	const finishCooking = (values: CookDoneValues, changed: boolean) => {
		if (!changed) {
			void exit();
			return;
		}
		updateRecipe.mutate(
			{ ...recipeToInput(recipe), rating: values.rating, notes: values.notes || null },
			{ onSuccess: () => void exit() },
		);
	};

	// Arrow keys step through the recipe and Escape leaves cook mode.
	useEffect(() => {
		if (total === 0) return;
		const onKeyDown = (event: KeyboardEvent) => {
			if (event.key === "ArrowLeft") {
				if (isDone) void goTo(lastStep);
				else if (current > 0) void goTo(current - 1);
			} else if (event.key === "ArrowRight" && !isDone) next();
			else if (event.key === "Escape") void exit();
		};
		window.addEventListener("keydown", onKeyDown);
		return () => window.removeEventListener("keydown", onKeyDown);
	});

	if (total === 0) {
		return <div className={classes.empty}>
			<Button variant="default" leftSection={<IconChevronLeft size={16} stroke={1.5} />} onClick={exit}>
				Back to recipe
			</Button>
			<Alert color="red" title="Nothing to cook from">This recipe has no instructions yet.</Alert>
		</div>;
	}

	const toggleIngredient = (index: number) => {
		setChecked((previous) => (
			previous.includes(index) ? previous.filter((item) => item !== index) : [...previous, index]
		));
	};

	// Timers belong to their step, so they keep running when the user moves to another step.
	const updateTimer = (id: string, change: (timer: CookTimer, now: number) => CookTimer, fallback: CookTimer) => {
		const now = Date.now();
		setTimers((previous) => ({ ...previous, [id]: change(previous[id] ?? fallback, now) }));
	};
	const stepTimers = timersForStep(current, recipe.instructions[current]);

	const textScaleStyle = { "--cook-text-scale": TEXT_SCALE_FACTORS[config["cook-text-scale"]] } as CSSProperties;

	return <div className={classes.view} style={textScaleStyle}>
		<CookTopBar
			recipeName={recipe.name}
			current={current}
			done={done}
			total={total}
			minutesLeft={isDone ? 0 : timeLeft(recipe.instructions, current)}
			keepAwake={config["cook-keep-awake"]}
			textScale={config["cook-text-scale"]}
			onExit={exit}
			onKeepAwakeChange={(value) => setConfig.mutate({ key: "cook-keep-awake", value })}
			onTextScaleChange={(value) => setConfig.mutate({ key: "cook-text-scale", value })}
		/>

		<div className={classes.body}>
			<aside className={classes.side}>
				<IngredientChecklist
					ingredients={recipe.ingredients}
					checked={checked}
					onToggle={toggleIngredient}
				/>
			</aside>

			<main className={classes.centre}>
				{isDone
					? <CookDone
						recipeName={recipe.name}
						initial={{ rating: recipe.rating ?? null, notes: recipe.notes ?? "" }}
						saving={updateRecipe.isPending}
						error={updateRecipe.isError ? String(updateRecipe.error) : null}
						onFinish={finishCooking}
						onBackToSteps={() => void goTo(lastStep)}
					/>
					: <>
						<StepPanel index={current} total={total} text={recipe.instructions[current]} />
						<div className={classes.timers}>
							{stepTimers.map((def) => {
								const fallback = newTimer(def);
								return <TimerCard
									key={def.id}
									timer={timers[def.id] ?? fallback}
									onStart={() => updateTimer(def.id, startTimer, fallback)}
									onPause={() => updateTimer(def.id, pauseTimer, fallback)}
									onAddMinute={() => updateTimer(def.id, addMinute, fallback)}
									onReset={() => updateTimer(def.id, resetTimer, fallback)}
					onMinutesChange={(minutes) => updateTimer(def.id, (timer) => setTimerMinutes(timer, minutes), fallback)}
								/>;
							})}
						</div>
						<CookNav
							canPrevious={current > 0}
							nextLabel={current === lastStep ? "Finish" : "Next step"}
							onPrevious={() => void goTo(current - 1)}
							onNext={next}
						/>
					</>}
			</main>

			<aside className={classes.side}>
				<StepList
					steps={recipe.instructions}
					current={current}
					done={done}
					onSelect={(index) => void goTo(index)}
				/>
			</aside>
		</div>
	</div>;
}

import { Fragment } from "react";
import { IconPlus } from "@tabler/icons-react";
import { Text, UnstyledButton } from "@mantine/core";
import dayjs from "../../../lib/dayjs";
import { openAddMealModal } from "../modals/AddMealModal";
import { repeatsOnDay } from "../trackSettings";
import type { PlanByDate, PlannedMeal, Track } from "../types";
import { DATE_FORMAT } from "../week";
import classes from "./PlannerSchedule.module.css";

interface PlannerScheduleProps {
	weekStart: dayjs.Dayjs;
	tracks: Track[];
	plan: PlanByDate;
}

export function PlannerSchedule({ weekStart, tracks, plan }: PlannerScheduleProps) {
	const days = Array.from({ length: 7 }, (_, i) => weekStart.add(i, "day"));
	const today = dayjs();

	return <div className={classes.grid}>
		<div />
		{days.map((day) => {
			const isToday = day.isSame(today, "day");

			return <div key={day.format("YYYY-MM-DD")} className={isToday ? classes.dayHeaderToday : classes.dayHeader}>
				<Text size="sm" c={isToday ? undefined : "dimmed"}>{day.format("ddd")}</Text>
				<Text size="lg" fw={700}>{day.format("D")}</Text>
			</div>
		})}

		{tracks.filter((track) => track.visible).map((track) => (
			<Fragment key={track.key}>
				<div className={classes.trackHeader}>
					<Text className={classes.trackLabel} c={track.color}>{track.label}</Text>
					<Text size="xs" c="dimmed">{track.defaultTime ?? "Anytime"}</Text>
				</div>

				{days.map((day) => {
					const date = day.format(DATE_FORMAT);
					const meal = plan[date]?.[track.key];
					const isToday = day.isSame(today, "day");

					// Keeps the grid aligned on days this track does not repeat.
					if (!meal && !repeatsOnDay(track, day)) {
						return <div key={date} />;
					}

					const title = `${track.label} · ${day.format("ddd D MMM")}`;

					return meal
						? <RecipeCard
							key={date}
							meal={meal}
							highlighted={isToday}
							onEdit={() => openAddMealModal({ title, date, trackKey: track.key, recipeId: meal.recipeId })}
						/>
						: <AddMealCell
							key={date}
							highlighted={isToday}
							onAdd={() => openAddMealModal({ title, date, trackKey: track.key })}
						/>
				})}
			</Fragment>
		))}
	</div>
}

interface RecipeCardProps {
	meal: PlannedMeal;
	highlighted: boolean;
	onEdit: () => void;
}

function RecipeCard({ meal, highlighted, onEdit }: RecipeCardProps) {
	const className = highlighted
		? `${classes.card} ${classes.editableCard} ${classes.cardToday}`
		: `${classes.card} ${classes.editableCard}`;

	return <UnstyledButton onClick={onEdit} className={className} aria-label={`Edit ${meal.name}`}>
		<Text size="sm" fw={600}>{meal.name}</Text>
		<Text size="xs" c="dimmed">
			{meal.minutes ?? "–"} min · {meal.rating ?? "–"} ★
		</Text>
	</UnstyledButton>
}

interface AddMealCellProps {
	highlighted: boolean;
	onAdd: () => void;
}

function AddMealCell({ highlighted, onAdd }: AddMealCellProps) {
	return <UnstyledButton onClick={onAdd} className={highlighted ? `${classes.emptyCell} ${classes.cardHighlighted}` : classes.emptyCell}>
		<IconPlus size={14} stroke={1.5} />
		<Text size="sm" c="dimmed">Add meal</Text>
	</UnstyledButton>
}

import { useState } from "react";
import { Text, UnstyledButton } from "@mantine/core";
import dayjs from "../../../lib/dayjs";
import { openAddMealModal } from "../modals/AddMealModal";
import { pickDay } from "../dayList";
import { repeatsOnDay } from "../trackSettings";
import type { PlanByDate, Track } from "../types";
import { DATE_FORMAT } from "../week";
import { AddMealCell, RecipeCard } from "./PlannerSchedule";
import classes from "./PlannerDayList.module.css";

interface PlannerDayListProps {
	weekStart: dayjs.Dayjs;
	tracks: Track[];
	plan: PlanByDate;
}

/**
 * The Schedule on narrow screens: a strip of the week's days and, for the chosen day, one row per
 * visible track. The wide grid is in `PlannerSchedule`.
 */
export function PlannerDayList({ weekStart, tracks, plan }: PlannerDayListProps) {
	const [picked, setPicked] = useState<string | null>(null);
	const today = dayjs();
	const days = Array.from({ length: 7 }, (_, i) => weekStart.add(i, "day"));
	const day = pickDay(days, picked, today);
	const date = day.format(DATE_FORMAT);
	const isToday = day.isSame(today, "day");
	const visibleTracks = tracks.filter((track) => track.visible);

	return <div className={classes.list}>
		<div className={classes.strip}>
			{days.map((item) => {
				const key = item.format(DATE_FORMAT);
				const selected = key === date;
				const className = [
					classes.day,
					selected && classes.daySelected,
					item.isSame(today, "day") && classes.dayToday,
				].filter(Boolean).join(" ");

				return <UnstyledButton key={key} className={className} aria-pressed={selected} onClick={() => setPicked(key)}>
					<span className={classes.dayName}>{item.format("ddd")}</span>
					<span className={classes.dayNumber}>{item.format("D")}</span>
				</UnstyledButton>
			})}
		</div>

		<Text fw={700} size="lg" className={classes.heading}>
			{isToday ? `Today · ${day.format("ddd D MMM")}` : day.format("dddd D MMM")}
		</Text>

		<div className={classes.items}>
			{visibleTracks.map((track) => {
				const meal = plan[date]?.[track.key];
				// Skips open slots on days this track does not repeat, as the grid does.
				if (!meal && !repeatsOnDay(track, day)) return null;

				const title = `${track.label} · ${day.format("ddd D MMM")}`;

				return <div key={track.key} className={classes.item}>
					<Text className={classes.trackLabel} c={track.color}>{track.label}</Text>
					{meal
						? <RecipeCard
							meal={meal}
							highlighted={false}
							onEdit={() => openAddMealModal({ title, date, trackKey: track.key, recipeId: meal.recipeId })}
						/>
						: <AddMealCell highlighted={false} onAdd={() => openAddMealModal({ title, date, trackKey: track.key })} />}
				</div>
			})}
		</div>

		{visibleTracks.length === 0 && <Text c="dimmed" size="sm">No tracks yet. Add one from Tracks.</Text>}
	</div>
}

import { Text, Tooltip } from "@mantine/core";
import dayjs from "../../../lib/dayjs";
import { formatWeekRange } from "../format";
import type { DayPlan, PlanByDate, Track } from "../types";
import { PAST_WEEK_COUNT } from "../week";
import classes from "./PastWeeks.module.css";

interface PastWeeksProps {
	weekStart: dayjs.Dayjs;
	tracks: Track[];
	plan: PlanByDate;
}

export function PastWeeks({ weekStart, tracks, plan }: PastWeeksProps) {
	const pastWeekStarts = Array.from({ length: PAST_WEEK_COUNT }, (_, i) => weekStart.subtract(i + 1, "week"));

	return <div className={classes.panel}>
		<Text size="lg" fw={600} mb="md">Past weeks</Text>

		<div className={classes.rows}>
			{pastWeekStarts.map((start) => (
				<div key={start.format("YYYY-MM-DD")} className={classes.row}>
					<Text size="sm" className={classes.label}>{formatWeekRange(start)}</Text>

					<div className={classes.days}>
						{Array.from({ length: 7 }, (_, i) => {
							const day = start.add(i, "day");
							const dayPlan = plan[day.format("YYYY-MM-DD")];
							const hasMeals = Object.values(dayPlan ?? {}).some(Boolean);

							return <Tooltip
								key={day.format("YYYY-MM-DD")}
								label={<DayPlanSummary day={day} tracks={tracks} dayPlan={dayPlan} />}
								multiline
								withArrow
							>
								<div className={hasMeals ? classes.dayFilled : classes.dayEmpty} />
							</Tooltip>
						})}
					</div>
				</div>
			))}
		</div>
	</div>
}

interface DayPlanSummaryProps {
	day: dayjs.Dayjs;
	tracks: Track[];
	dayPlan?: DayPlan;
}

function DayPlanSummary({ day, tracks, dayPlan }: DayPlanSummaryProps) {
	return <>
		<Text size="xs" fw={600}>{day.format("ddd D MMM")}</Text>
		{tracks.map((track) => (
			<Text key={track.key} size="xs">
				{track.label}: {dayPlan?.[track.key]?.name ?? "Nothing planned"}
			</Text>
		))}
	</>
}

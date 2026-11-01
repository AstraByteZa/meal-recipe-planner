import { useNavigate, useSearch } from "@tanstack/react-router";
import { useSuspenseQuery } from "@tanstack/react-query";
import { IconAdjustmentsHorizontal, IconChevronLeft, IconChevronRight } from "@tabler/icons-react";
import { ActionIcon, Button, Group, Text, Title } from "@mantine/core";
import dayjs from "../../../lib/dayjs";
import { FillTheGaps } from "../components/FillTheGaps";
import { openManageTracksModal } from "../modals/ManageTracksModal";
import { PastWeeks } from "../components/PastWeeks";
import { PlannerDayList } from "../components/PlannerDayList";
import { PlannerSchedule } from "../components/PlannerSchedule";
import { RemovedNotice } from "../../sync/components/RemovedBanner";
import { SignedOutBanner } from "../../sync/components/SignedOutBanner";
import { scheduleQueryOptions, tracksQueryOptions } from "../queries";
import { formatWeekRange } from "../format";
import { currentWeek, shiftWeek } from "../week";
import classes from "./MealPlannerView.module.css";

export function MealPlannerView() {
	const { week } = useSearch({ from: "/" });
	const navigate = useNavigate({ from: "/" });
	const { data: tracks } = useSuspenseQuery(tracksQueryOptions);
	const { data: plan } = useSuspenseQuery(scheduleQueryOptions(week));

	const weekStart = dayjs(week);
	// ISO week-numbering year, taken from the Thursday of the week.
	const isoYear = weekStart.add(3, "day").year();

	return <div className={classes.view}>
		<RemovedNotice />
		<SignedOutBanner />

		<header className={classes.header}>
			<div>
				<Text size="xs" c="dimmed" className={classes.eyebrow}>
					Week {weekStart.isoWeek()} · {isoYear}
				</Text>
				<Title order={1}>{formatWeekRange(weekStart)}</Title>
			</div>

			<Group gap="xs">
				<ActionIcon variant="default" size="lg" aria-label="Previous week" onClick={() => navigate({ search: { week: shiftWeek(week, -1) } })}>
					<IconChevronLeft size={16} stroke={1.5} />
				</ActionIcon>
				<Button variant="default" onClick={() => navigate({ search: { week: currentWeek() } })}>Today</Button>
				<ActionIcon variant="default" size="lg" aria-label="Next week" onClick={() => navigate({ search: { week: shiftWeek(week, 1) } })}>
					<IconChevronRight size={16} stroke={1.5} />
				</ActionIcon>
				<Button variant="default" leftSection={<IconAdjustmentsHorizontal size={16} stroke={1.5} />} onClick={openManageTracksModal}>Tracks</Button>
				<Button className={classes.wideOnly}>Auto-fill gaps</Button>
			</Group>
		</header>

		<div className={classes.wideOnly}>
			<PlannerSchedule weekStart={weekStart} tracks={tracks} plan={plan} />
		</div>
		<PlannerDayList weekStart={weekStart} tracks={tracks} plan={plan} />

		<div className={classes.lower}>
			<PastWeeks weekStart={weekStart} tracks={tracks} plan={plan} />
			<FillTheGaps />
		</div>
	</div>
}

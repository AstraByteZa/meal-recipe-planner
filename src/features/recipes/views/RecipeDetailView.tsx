import { useState } from "react";
import { useSuspenseQuery } from "@tanstack/react-query";
import { Link, useNavigate } from "@tanstack/react-router";
import { IconChevronLeft, IconExternalLink } from "@tabler/icons-react";
import { openUrl } from "@tauri-apps/plugin-opener";
import { Alert, Badge, Button, Group, Image, Text, Title, Tooltip, UnstyledButton } from "@mantine/core";
import dayjs from "../../../lib/dayjs";
import { useSetScheduleEntry } from "../../planner/mutations";
import { tracksQueryOptions } from "../../planner/queries";
import { currentWeek, DATE_FORMAT } from "../../planner/week";
import { StarRating } from "../components/StarRating";
import { formatDaysAgo } from "../format";
import { recipeQueryOptions } from "../queries";
import classes from "./RecipeDetailView.module.css";

interface RecipeDetailViewProps {
	recipeId: string;
}

export function RecipeDetailView({ recipeId }: RecipeDetailViewProps) {
	const { data: recipe } = useSuspenseQuery(recipeQueryOptions(recipeId));
	const { data: tracks } = useSuspenseQuery(tracksQueryOptions);
	const navigate = useNavigate();
	const setEntry = useSetScheduleEntry();

	const visibleTracks = tracks.filter((track) => track.visible);
	const weekStart = dayjs(currentWeek());
	const days = Array.from({ length: 7 }, (_, i) => weekStart.add(i, "day"));

	const [dayIndex, setDayIndex] = useState(dayjs().isoWeekday() - 1);
	const [trackKey, setTrackKey] = useState(recipe.usualTrackKey ?? visibleTracks[0]?.key ?? "");

	const selectedDay = days[dayIndex];
	const selectedTrack = tracks.find((track) => track.key === trackKey);
	const usualTrack = tracks.find((track) => track.key === recipe.usualTrackKey);
	const canCook = recipe.instructions.length > 0;

	const addToPlan = () => {
		if (!selectedTrack) return;

		setEntry.mutate({
			date: selectedDay.format(DATE_FORMAT),
			trackKey: selectedTrack.key,
			recipeId: recipe.id,
		});
	};

	return <div className={classes.view}>
		<header className={classes.header}>
			<div>
				<Link to="/recipes" className={classes.back}>
					<IconChevronLeft size={14} stroke={1.5} />
					Recipe links
				</Link>
				<Title order={1} mt="xs">{recipe.name}</Title>
				<Group gap="xs" mt="sm">
					{recipe.url
						? <UnstyledButton className={classes.source} onClick={() => void openUrl(recipe.url)}>
							{recipe.sourceDomain}
							<IconExternalLink size={14} stroke={1.5} />
						</UnstyledButton>
						: <Text size="sm" c="dimmed">No link yet</Text>}
					{recipe.tags.map((tag) => <Badge key={tag} variant="default" radius="sm" size="md">{tag}</Badge>)}
				</Group>
			</div>

			<Group gap="sm" wrap="nowrap">
				<Button
					variant="default"
					onClick={() => navigate({ to: "/recipes/$recipeId/edit", params: { recipeId: recipe.id } })}
				>
					Edit
				</Button>
				<Tooltip label="Add instructions to cook this recipe" disabled={canCook} withArrow>
					<span>
						<Button
							onClick={() => navigate({ to: "/recipes/$recipeId/cook", params: { recipeId: recipe.id }, search: { step: 0 } })}
							disabled={!canCook}
						>
							Cook now
						</Button>
					</span>
				</Tooltip>
				<Button onClick={addToPlan} loading={setEntry.isPending} disabled={!selectedTrack}>
					Add to plan
				</Button>
			</Group>
		</header>

		<div className={classes.stats}>
			<div className={classes.stat}>
				<Text size="xs" c="dimmed" className={classes.statLabel}>Rating</Text>
				<StarRating value={recipe.rating} />
				<Text size="sm" c="dimmed" mt={6}>{recipe.rating ? `${recipe.rating} out of 5` : "Not rated yet"}</Text>
			</div>

			<div className={classes.stat}>
				<Text size="xs" c="dimmed" className={classes.statLabel}>Last used</Text>
				{recipe.lastUsedDaysAgo === null ? <>
					<Text className={classes.statValue}>Never</Text>
					<Text size="sm" c="dimmed">Not planned yet</Text>
				</> : <>
					<Text className={classes.statValue}>{dayjs().subtract(recipe.lastUsedDaysAgo, "day").format("ddd D MMM")}</Text>
					<Text size="sm" c="dimmed">{recipe.lastUsedDaysAgo === 0 ? "Today, in this week's plan" : formatDaysAgo(recipe.lastUsedDaysAgo)}</Text>
				</>}
			</div>

			<div className={classes.stat}>
				<Text size="xs" c="dimmed" className={classes.statLabel}>Planned</Text>
				<Text className={classes.statValue}>{recipe.plannedCount}×</Text>
				<Text size="sm" c="dimmed">Since you saved it</Text>
			</div>

			<div className={classes.stat}>
				<Text size="xs" c="dimmed" className={classes.statLabel}>Saved</Text>
				<Text className={classes.statValue}>{dayjs().subtract(recipe.savedDaysAgo, "day").format("D MMM")}</Text>
				<Text size="sm" c="dimmed">{formatDaysAgo(recipe.savedDaysAgo)}</Text>
			</div>
		</div>

		<div className={classes.body}>
			<div className={classes.column}>
				{recipe.imageUrl && <section className={`${classes.panel} ${classes.photoPanel}`}>
					<div className={classes.photo}>
						<Image src={recipe.imageUrl} alt={recipe.name} />
						{recipe.sourceDomain && <Text size="xs" className={classes.caption}>Image from {recipe.sourceDomain}</Text>}
					</div>
				</section>}

				{recipe.ingredients.length > 0 && <section className={classes.panel}>
					<div className={classes.panelHeader}>
						<Text className={classes.panelTitle}>Ingredients</Text>
						<Text size="sm" c="dimmed">
							{[recipe.servings && `Serves ${recipe.servings}`, `${recipe.ingredients.length} items`].filter(Boolean).join(" · ")}
						</Text>
					</div>
					<ul className={classes.ingredients}>
						{recipe.ingredients.map((ingredient, index) => <li key={index}>{ingredient}</li>)}
					</ul>
				</section>}

				{recipe.instructions.length > 0 && <section className={classes.panel}>
					<Text className={classes.panelTitle}>Instructions</Text>
					<ol className={classes.steps}>
						{recipe.instructions.map((step, index) => <li key={index} className={classes.step}>
								<span className={classes.stepNumber}>{index + 1}</span>
								<Text className={classes.stepText}>{step}</Text>
							</li>)}
					</ol>
				</section>}

				<section className={classes.panel}>
					<Text className={classes.panelTitle}>Notes</Text>
					{recipe.notes
						? <Text className={classes.notes}>{recipe.notes}</Text>
						: <Text size="sm" c="dimmed">No notes yet.</Text>}
				</section>

				<section className={classes.panel}>
					<Text className={classes.panelTitle}>Details</Text>
					<div className={classes.details}>
						<div>
							<Text size="xs" c="dimmed" className={classes.statLabel}>Cook time</Text>
							<Text className={classes.statValue}>{recipe.minutes ? `${recipe.minutes} min` : "–"}</Text>
						</div>
						<div>
							<Text size="xs" c="dimmed" className={classes.statLabel}>Serves</Text>
							<Text className={classes.statValue}>{recipe.servings ?? "–"}</Text>
						</div>
						<div>
							<Text size="xs" c="dimmed" className={classes.statLabel}>Usually planned for</Text>
							<Text className={classes.statValue}>{usualTrack?.label ?? "Not set"}</Text>
						</div>
					</div>
				</section>
			</div>

			<div className={classes.column}>
				<section className={classes.panel}>
					<Text className={classes.panelTitle}>Add to plan</Text>

					<Text size="xs" c="dimmed" className={classes.sectionLabel}>Day this week</Text>
					<div className={classes.chips}>
						{days.map((day, index) => (
							<UnstyledButton
								key={day.format(DATE_FORMAT)}
								aria-pressed={index === dayIndex}
								onClick={() => setDayIndex(index)}
								className={index === dayIndex ? `${classes.chip} ${classes.chipActive}` : classes.chip}
							>
								{day.format("ddd")}
							</UnstyledButton>
						))}
					</div>

					<Text size="xs" c="dimmed" className={classes.sectionLabel}>Track</Text>
					<div className={classes.chips}>
						{visibleTracks.map((track) => (
							<UnstyledButton
								key={track.key}
								aria-pressed={track.key === trackKey}
								onClick={() => setTrackKey(track.key)}
								className={track.key === trackKey ? `${classes.chip} ${classes.chipActive}` : classes.chip}
							>
								{track.label}
							</UnstyledButton>
						))}
					</div>

					<Button
						fullWidth
						mt="md"
						onClick={addToPlan}
						loading={setEntry.isPending}
						disabled={!selectedTrack}
					>
						{selectedTrack ? `Add to ${selectedDay.format("dddd")} ${selectedTrack.label.toLowerCase()}` : "Add to plan"}
					</Button>

					{setEntry.isError && <Alert color="red" title="Could not add to the plan" mt="md">{String(setEntry.error)}</Alert>}
				</section>

				<section className={classes.panel}>
					<Text className={classes.panelTitle}>Plan history</Text>
					{recipe.history.length === 0
						? <Text size="sm" c="dimmed">Not planned yet.</Text>
						: <div className={classes.history}>
							{recipe.history.map((entry) => (
								<div key={`${entry.date}-${entry.trackKey}`} className={classes.historyRow}>
									<Text size="sm">{dayjs(entry.date).format("ddd D MMM")}</Text>
									<Text size="xs" c="dimmed">{entry.trackLabel} · week {dayjs(entry.date).isoWeek()}</Text>
								</div>
							))}
						</div>}
				</section>
			</div>
		</div>
	</div>
}

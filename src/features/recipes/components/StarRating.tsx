import { IconStar, IconStarFilled } from "@tabler/icons-react";
import { Group, Text, UnstyledButton } from "@mantine/core";
import classes from "./StarRating.module.css";

const MAX_RATING = 5;

interface StarRatingProps {
	value?: number | null;
}

/** Read-only rating. Shows "Not rated" when there is no value. */
export function StarRating({ value }: StarRatingProps) {
	if (value == null) {
		return <Text size="sm" c="dimmed">Not rated</Text>;
	}

	return <Group gap={2} wrap="nowrap" aria-label={`${value} of ${MAX_RATING} stars`}>
		{Array.from({ length: MAX_RATING }, (_, i) => i < value
			? <IconStarFilled key={i} size={14} className={classes.starOn} />
			: <IconStar key={i} size={14} stroke={1.5} className={classes.starOff} />
		)}
	</Group>
}

interface StarInputProps {
	value: number | null;
	onChange: (value: number | null) => void;
}

/** Clickable rating. Clicking the current rating clears it. */
export function StarInput({ value, onChange }: StarInputProps) {
	return <Group gap={4} wrap="nowrap">
		{Array.from({ length: MAX_RATING }, (_, i) => {
			const star = i + 1;
			const isOn = value !== null && star <= value;

			return <UnstyledButton
				key={star}
				aria-label={`${star} of ${MAX_RATING} stars`}
				aria-pressed={isOn}
				onClick={() => onChange(value === star ? null : star)}
				className={classes.inputStar}
			>
				{isOn
					? <IconStarFilled size={22} className={classes.starOn} />
					: <IconStar size={22} stroke={1.5} className={classes.starOff} />}
			</UnstyledButton>
		})}
		{value !== null && <Text size="sm" c="dimmed" ml="xs">{value} of {MAX_RATING}</Text>}
	</Group>
}

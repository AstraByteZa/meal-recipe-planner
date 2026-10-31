import { Button, Group } from "@mantine/core";
import classes from "./StepFooter.module.css";

interface StepFooterProps {
	onBack?: () => void;
	onSkip?: () => void;
	onNext: () => void;
	nextLabel?: string;
	nextLoading?: boolean;
}

/** Back on the left. Skip and the forward action on the right. */
export function StepFooter({ onBack, onSkip, onNext, nextLabel = "Continue", nextLoading }: StepFooterProps) {
	return <footer className={classes.footer}>
		{onBack ? <Button variant="default" onClick={onBack}>Back</Button> : <span />}
		<Group gap="md">
			{onSkip && <Button variant="subtle" color="gray" onClick={onSkip}>Skip for now</Button>}
			<Button onClick={onNext} loading={nextLoading}>{nextLabel}</Button>
		</Group>
	</footer>
}

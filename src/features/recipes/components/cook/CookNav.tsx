import { IconChevronLeft, IconChevronRight } from "@tabler/icons-react";
import { Button } from "@mantine/core";
import classes from "./CookNav.module.css";

interface CookNavProps {
	canPrevious: boolean;
	nextLabel: string;
	onPrevious: () => void;
	onNext: () => void;
}

export function CookNav({ canPrevious, nextLabel, onPrevious, onNext }: CookNavProps) {
	return <div className={classes.nav}>
		<Button
			variant="default"
			size="lg"
			leftSection={<IconChevronLeft size={18} stroke={1.5} />}
			disabled={!canPrevious}
			onClick={onPrevious}
		>
			Previous step
		</Button>
		<Button
			size="lg"
			className={classes.next}
			rightSection={<IconChevronRight size={18} stroke={1.5} />}
			onClick={onNext}
		>
			{nextLabel}
		</Button>
	</div>;
}

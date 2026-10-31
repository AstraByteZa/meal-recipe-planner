import { IconCheck } from "@tabler/icons-react";
import { Text, UnstyledButton } from "@mantine/core";
import classes from "./StepList.module.css";

interface StepListProps {
	steps: string[];
	current: number;
	done: number[];
	onSelect: (index: number) => void;
}

export function StepList({ steps, current, done, onSelect }: StepListProps) {
	return <section className={classes.panel}>
		<Text className={classes.title}>All steps</Text>
		<ol className={classes.list}>
			{steps.map((step, index) => {
				const isDone = done.includes(index);
				const isCurrent = index === current;
				return <li key={index}>
					<UnstyledButton
						className={isCurrent ? `${classes.item} ${classes.itemCurrent}` : classes.item}
						aria-current={isCurrent ? "step" : undefined}
						onClick={() => onSelect(index)}
					>
						<span className={isDone ? `${classes.marker} ${classes.markerDone}` : classes.marker}>
							{isDone ? <IconCheck size={12} stroke={2.5} /> : index + 1}
						</span>
						<span className={isDone ? classes.doneText : undefined}>{step}</span>
					</UnstyledButton>
				</li>;
			})}
		</ol>
	</section>;
}

import { Checkbox, Text } from "@mantine/core";
import classes from "./IngredientChecklist.module.css";

interface IngredientChecklistProps {
	ingredients: string[];
	checked: number[];
	onToggle: (index: number) => void;
}

export function IngredientChecklist({ ingredients, checked, onToggle }: IngredientChecklistProps) {
	return <section className={classes.panel}>
		<div className={classes.header}>
			<Text className={classes.title}>Ingredients</Text>
			<Text size="sm" c="dimmed">{checked.length} of {ingredients.length} used</Text>
		</div>
		<ul className={classes.list}>
			{ingredients.map((ingredient, index) => {
				const isChecked = checked.includes(index);
				return <li key={index}>
					<Checkbox
						checked={isChecked}
						onChange={() => onToggle(index)}
						label={<span className={isChecked ? `${classes.ingredient} ${classes.used}` : classes.ingredient}>{ingredient}</span>}
					/>
				</li>;
			})}
		</ul>
	</section>;
}

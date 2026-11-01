import type { ReactNode } from "react";
import { IconCalendarWeek, IconCheck } from "@tabler/icons-react";
import { Text, Title } from "@mantine/core";
import classes from "./AuthLayout.module.css";

export interface AuthPitch {
	title: string;
	text: string;
	points: string[];
}

interface AuthLayoutProps {
	pitch: AuthPitch;
	children: ReactNode;
}

/**
 * Full-page layout for sign-in and invites: the pitch on the left, the form card on the right.
 * On narrow screens the pitch sits above the card.
 */
export function AuthLayout({ pitch, children }: AuthLayoutProps) {
	return <div className={classes.layout}>
		<aside className={classes.pitch}>
			<div className={classes.brand}>
				<span className={classes.mark}>
					<IconCalendarWeek size={16} stroke={1.8} />
				</span>
				<Text fw={700}>Mise</Text>
			</div>
			<Title order={1} className={classes.title}>{pitch.title}</Title>
			<Text c="dimmed">{pitch.text}</Text>
			<ul className={classes.points}>
				{pitch.points.map((point) => <li key={point}>
					<IconCheck size={16} stroke={2} className={classes.check} />
					<Text size="sm">{point}</Text>
				</li>)}
			</ul>
		</aside>

		<main className={classes.main}>
			<div className={classes.card}>{children}</div>
		</main>
	</div>;
}

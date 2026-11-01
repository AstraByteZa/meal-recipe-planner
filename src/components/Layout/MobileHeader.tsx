import { IconCircleDot } from "@tabler/icons-react";
import { Box, Text } from "@mantine/core";
import { SyncChip } from "../../features/sync/components/SyncChip";
import classes from "./MobileHeader.module.css";

/** Top bar on narrow screens. The sidebar's brand and sync chip move here. */
export function MobileHeader() {
	return <header className={classes.header}>
		<div className={classes.brand}>
			<Box className={classes.logo}>
				<IconCircleDot size={16} stroke={2} />
			</Box>
			<Text fw={700} size="lg">Mise</Text>
		</div>
		<SyncChip compact />
	</header>
}

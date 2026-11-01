import { Group } from "@mantine/core";
import { Outlet } from "@tanstack/react-router";
import { NavbarSearch } from "../NavbarSearch/NavbarSearch";
import { BottomTabs } from "./BottomTabs";
import { MobileHeader } from "./MobileHeader";
import classes from "./DashboardLayout.module.css";

/**
 * The signed-in shell. Wide screens get the sidebar. Narrow screens get a header with the sync
 * chip and a bottom tab bar instead. Both are shown or hidden in CSS, so no JS checks the width.
 */
export function DashboardLayout() {
	return <Group mih={"100vh"} miw={"100vw"} align="stretch">
		<NavbarSearch />
		<div className={classes.column}>
			<MobileHeader />
			<div className={classes.content}>
				<Outlet />
			</div>
		</div>
		<BottomTabs />
	</Group>
}

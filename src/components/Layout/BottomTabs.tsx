import { useQuery } from "@tanstack/react-query";
import { Link } from "@tanstack/react-router";
import { syncStatusQueryOptions } from "../../features/sync/queries";
import { navItems } from "./navItems";
import classes from "./BottomTabs.module.css";

/** Main navigation on narrow screens. The sidebar does this job on wide ones. */
export function BottomTabs() {
	const { data: status } = useQuery(syncStatusQueryOptions);

	return <nav className={classes.tabs} aria-label="Main">
		{navItems(status?.mode === "signed_in").map((item) => (
			<Link
				key={item.label}
				to={item.to}
				activeOptions={{ exact: item.to === "/" }}
				className={classes.tab}
				activeProps={{ className: `${classes.tab} ${classes.tabActive}` }}
			>
				<item.icon size={22} stroke={1.5} />
				<span>{item.label}</span>
			</Link>
		))}
	</nav>
}

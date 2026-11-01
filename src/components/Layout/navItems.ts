import { IconCalendarWeek, IconLink, IconUsers } from "@tabler/icons-react";

export type NavTo = "/" | "/recipes" | "/family";

export interface NavItem {
	icon: typeof IconCalendarWeek;
	label: string;
	to: NavTo;
}

/** The main destinations, shared by the sidebar and the bottom tab bar. */
export function navItems(signedIn: boolean): NavItem[] {
	const items: NavItem[] = [
		{ icon: IconCalendarWeek, label: "Schedule", to: "/" },
		{ icon: IconLink, label: "Recipe links", to: "/recipes" },
	];
	// Family only exists for signed-in accounts.
	if (signedIn) items.push({ icon: IconUsers, label: "Family", to: "/family" });
	return items;
}

import { Group } from "@mantine/core";
import { NavbarSearch } from "../NavbarSearch/NavbarSearch";
import { Outlet } from "@tanstack/react-router";

export function DashboardLayout() {

	return <Group mih={"100vh"} miw={"100vw"} align="stretch">
		<NavbarSearch />
		{/*<Link to="/" className="[&.active]:font-bold">
          Home
        </Link>{' '}
        <Link to="/about" className="[&.active]:font-bold">
          About
        </Link>*/}
		<Outlet />
	</Group>
}

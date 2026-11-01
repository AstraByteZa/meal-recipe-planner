import { useState } from "react";
import { IconCloudOff, IconX } from "@tabler/icons-react";
import { ActionIcon, Button, Group, Text } from "@mantine/core";
import { useQuery } from "@tanstack/react-query";
import { useNavigate } from "@tanstack/react-router";
import { syncStatusQueryOptions } from "../queries";
import classes from "./SignedOutBanner.module.css";

export const SIGNED_OUT_BANNER_KEY = "signedOutBannerDismissed";

/**
 * Shown on the Schedule while the device is not signed in. Dismissing it is remembered on this
 * device, so it does not return on the next launch.
 */
export function SignedOutBanner() {
	const { data: status } = useQuery(syncStatusQueryOptions);
	const navigate = useNavigate();
	const [dismissed, setDismissed] = useState(() => localStorage.getItem(SIGNED_OUT_BANNER_KEY) === "true");

	if (!status || status.mode === "signed_in" || dismissed) return null;

	const dismiss = () => {
		localStorage.setItem(SIGNED_OUT_BANNER_KEY, "true");
		setDismissed(true);
	};

	return <div className={classes.banner} role="status">
		<IconCloudOff size={18} stroke={1.5} />
		<Text className={classes.text}>
			You're not signed in. Your plans and recipes stay on this device. Sign in to sync them and share with a family.
		</Text>
		<Group gap="xs" wrap="nowrap">
			<Button size="xs" variant="default" onClick={() => navigate({ to: "/sign-in" })}>Sign in</Button>
			<ActionIcon variant="subtle" color="gray" aria-label="Dismiss" onClick={dismiss}>
				<IconX size={14} stroke={1.5} />
			</ActionIcon>
		</Group>
	</div>;
}

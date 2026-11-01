import { useNavigate } from "@tanstack/react-router";
import { useQuery } from "@tanstack/react-query";
import { Anchor, Button, Stack, Text, Title } from "@mantine/core";
import { AuthLayout } from "../../../components/AuthLayout/AuthLayout";
import { AccountForm } from "../components/AccountForm";
import { syncStatusQueryOptions } from "../queries";

interface SignInViewProps {
	/** The in-app page to return to after signing in. */
	next?: string;
}

export function SignInView({ next }: SignInViewProps) {
	const navigate = useNavigate();
	const { data: status } = useQuery(syncStatusQueryOptions);
	const goNext = () => navigate({ href: next ?? "/", replace: true });

	return <AuthLayout
		pitch={{
			title: "Your week, in sync.",
			text: "Plans and recipes live on this device first. Sign in only if you want them on more than one.",
			points: ["Works without an account", "Share with a family using a code", "No email invites, no spam"],
		}}
	>
		<Stack gap={4}>
			<Title order={2}>Sign in</Title>
			<Text size="sm" c="dimmed">Sync your plans and recipes across devices.</Text>
		</Stack>

		{status && <AccountForm
			initialServerUrl={status.serverUrl ?? status.defaultServerUrl ?? ""}
			advancedServer
			onSignedIn={goNext}
		/>}

		<Button variant="default" fullWidth onClick={goNext}>
			Continue without an account
		</Button>

		<Text ta="center">
			<Anchor component="button" type="button" fw={600} onClick={() => navigate({ to: "/join" })}>
				Have an invite code?
			</Anchor>
		</Text>
	</AuthLayout>;
}

import { useQuery, useSuspenseQuery } from "@tanstack/react-query";
import { useNavigate } from "@tanstack/react-router";
import { Alert, Avatar, Button, Group, Loader, Stack, Text, Title } from "@mantine/core";
import { FamilyStartCards } from "../../sync/components/FamilyStartCards";
import { RemovedBanner } from "../../sync/components/RemovedBanner";
import { familyDetailsQueryOptions, syncStatusQueryOptions } from "../../sync/queries";
import type { SyncStatus } from "../../sync/schema";
import { findSelf, initialOf, memberSummary } from "../format";
import { DangerZoneCard, LeaveCard } from "../components/FamilyActions";
import { InviteCard, PendingInvitesCard } from "../components/InvitesCards";
import { MembersCard } from "../components/MembersCard";
import { openRenameFamilyModal } from "../modals/FamilyModals";
import cards from "../components/FamilyCards.module.css";
import classes from "./FamilyView.module.css";

export function FamilyView() {
	const { data: status } = useSuspenseQuery(syncStatusQueryOptions);

	return <div className={classes.view}>
		<header className={classes.header}>
			<div>
				<Text size="xs" c="dimmed" className={classes.eyebrow}>Settings</Text>
				<Title order={1}>Family</Title>
			</div>
		</header>

		{status.notice && <RemovedBanner notice={status.notice} />}
		<FamilyBody status={status} />
	</div>;
}

function FamilyBody({ status }: { status: SyncStatus }) {
	if (status.mode !== "signed_in") return <SignedOutEmpty />;
	if (!status.family) return <NoFamily />;
	return <FamilyContent family={status.family} email={status.user?.email} />;
}

function SignedOutEmpty() {
	const navigate = useNavigate();

	return <section className={`${cards.panel} ${classes.single}`}>
		<Stack gap={4}>
			<Text fw={600} size="lg">Family sharing needs an account</Text>
			<Text size="sm" c="dimmed">
				Sign in to create a family or join one with an invite code. Until then, everything stays on this device.
			</Text>
		</Stack>
		<Group>
			<Button onClick={() => navigate({ to: "/sign-in", search: { next: "/family" } })}>Sign in</Button>
		</Group>
	</section>;
}

function NoFamily() {
	return <Stack gap="md" className={classes.single}>
		<section className={cards.panel}>
			<Stack gap={4}>
				<Text fw={600} size="lg">You're not in a family yet</Text>
				<Text size="sm" c="dimmed">
					Create one and invite people, or join with an invite code from the family owner.
				</Text>
			</Stack>
		</section>
		<FamilyStartCards />
	</Stack>;
}

/** The family's details. Owners see invites and the owner tools, members see Leave. */
function FamilyContent({ family, email }: { family: NonNullable<SyncStatus["family"]>; email?: string }) {
	const { data: details, isPending, isError, error } = useQuery(familyDetailsQueryOptions(family.id));

	if (isPending) return <Loader size="sm" />;
	if (isError) return <Alert color="red" title="Could not load the family">{String(error)}</Alert>;

	const isOwner = family.role === "owner";
	const self = findSelf(details.members, email);

	const header = <section className={cards.panel}>
		<Group justify="space-between" wrap="nowrap">
			<Group wrap="nowrap">
				<Avatar size={56} radius="md" color="myColor" variant="light">{initialOf(family.name)}</Avatar>
				<Stack gap={0}>
					<Title order={2}>{family.name}</Title>
					<Text size="sm" c="dimmed">{memberSummary(details.members.length, family.role)}</Text>
				</Stack>
			</Group>
			{isOwner && <Button variant="default" onClick={() => openRenameFamilyModal(family.name)}>Rename family</Button>}
		</Group>
	</section>;

	const members = <MembersCard
		members={details.members}
		selfId={self?.user_id}
		isOwner={isOwner}
		familyName={family.name}
	/>;

	if (!isOwner) {
		return <div className={classes.single}>
			{header}
			{members}
			<LeaveCard familyName={family.name} selfId={self?.user_id} />
		</div>;
	}

	return <div className={classes.columns}>
		<div className={classes.column}>
			{header}
			{members}
			<PendingInvitesCard invites={details.invites} />
		</div>
		<div className={classes.column}>
			<InviteCard openInvites={details.invites.length} />
			<DangerZoneCard familyName={family.name} />
		</div>
	</div>;
}

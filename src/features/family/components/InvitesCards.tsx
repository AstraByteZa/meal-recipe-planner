import { useState } from "react";
import { IconCheck, IconCopy } from "@tabler/icons-react";
import { useClipboard } from "@mantine/hooks";
import { Alert, Button, Group, Stack, Text } from "@mantine/core";
import { formatInviteCode, expiresIn } from "../../sync/format";
import { useFamilyInviteCreate, useFamilyInviteRevoke } from "../../sync/mutations";
import type { FamilyDetails, FamilyInvite } from "../../sync/schema";
import classes from "./FamilyCards.module.css";

/** The number of open invites the server allows at once. */
export const MAX_OPEN_INVITES = 10;

/** Creates a code and shows it once, with a copy button. Owner only. */
export function InviteCard({ openInvites }: { openInvites: number }) {
	const create = useFamilyInviteCreate();
	const clipboard = useClipboard({ timeout: 2000 });
	const [latest, setLatest] = useState<FamilyInvite | null>(null);
	const full = openInvites >= MAX_OPEN_INVITES;

	return <section className={classes.panel}>
		<Stack gap={4}>
			<Text fw={600} size="lg">Invite someone</Text>
			<Text size="sm" c="dimmed">Codes only. Each one works once and expires in 7 days.</Text>
		</Stack>

		<Group>
			<Button onClick={() => create.mutate(undefined, { onSuccess: (invite) => setLatest(invite) })} loading={create.isPending} disabled={full}>
				Invite someone
			</Button>
		</Group>
		{full && <Text size="xs" c="dimmed">You have {MAX_OPEN_INVITES} open invites. Revoke one to make another.</Text>}
		{create.isError && <Alert color="red" title="Could not create an invite">{String(create.error)}</Alert>}

		{latest && <div className={classes.codePanel}>
			<Text className={classes.code}>{formatInviteCode(latest.code)}</Text>
			<Group justify="space-between" wrap="nowrap">
				<Text size="sm" c="dimmed">{expiresIn(latest.expires_at, new Date())}, works once</Text>
				<Button
					variant="default"
					leftSection={clipboard.copied ? <IconCheck size={16} stroke={1.5} /> : <IconCopy size={16} stroke={1.5} />}
					onClick={() => clipboard.copy(formatInviteCode(latest.code))}
				>
					{clipboard.copied ? "Copied" : "Copy"}
				</Button>
			</Group>
		</div>}
	</section>;
}

/** Codes that have not been used, with Revoke. Owner only. */
export function PendingInvitesCard({ invites }: { invites: FamilyDetails["invites"] }) {
	const revoke = useFamilyInviteRevoke();
	const now = new Date();

	return <section className={classes.panel}>
		<Group gap="xs" align="baseline">
			<Text fw={600} size="lg">Pending invites</Text>
			<Text size="sm" c="dimmed">· {invites.length}</Text>
		</Group>

		{invites.length === 0 && <Text size="sm" c="dimmed">No open invites.</Text>}

		<div className={classes.list}>
			{invites.map((invite) => {
				const revokingThis = revoke.isPending && revoke.variables?.inviteId === invite.id;

				return <div key={invite.id} className={classes.row}>
					<Text className={classes.pendingCode}>{formatInviteCode(invite.code)}</Text>
					<Text size="sm" c="dimmed" className={classes.grow}>{expiresIn(invite.expires_at, now)}</Text>
					<Button
						variant="outline"
						color="red"
						size="xs"
						loading={revokingThis}
						onClick={() => revoke.mutate({ inviteId: invite.id })}
					>
						Revoke
					</Button>
				</div>;
			})}
		</div>
		{revoke.isError && <Alert color="red" title="Could not revoke the invite">{String(revoke.error)}</Alert>}
	</section>;
}

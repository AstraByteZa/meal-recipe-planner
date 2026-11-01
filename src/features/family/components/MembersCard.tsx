import { IconDots } from "@tabler/icons-react";
import { ActionIcon, Avatar, Badge, Group, Menu, Stack, Text } from "@mantine/core";
import type { FamilyDetails } from "../../sync/schema";
import { initialOf } from "../format";
import { openConfirmRemoveMemberModal, openConfirmTransferModal } from "../modals/FamilyModals";
import classes from "./FamilyCards.module.css";

type Member = FamilyDetails["members"][number];

/** Every member of the family. The owner can make someone else the owner, or remove them. */
export function MembersCard({
	members,
	selfId,
	isOwner,
	familyName,
}: {
	members: Member[];
	selfId?: string;
	isOwner: boolean;
	familyName: string;
}) {
	return <section className={classes.panel}>
		<Group gap="xs" align="baseline">
			<Text fw={600} size="lg">Members</Text>
			<Text size="sm" c="dimmed">· {members.length}</Text>
		</Group>

		<div className={classes.list}>
			{members.map((member) => {
				const isSelf = member.user_id === selfId;
				const isOther = isOwner && !isSelf;

				return <div key={member.user_id} className={classes.row}>
					<Avatar radius="xl" color="myColor" variant="light">{initialOf(member.name)}</Avatar>
					<Stack gap={0} className={classes.grow}>
						<Group gap={6} wrap="nowrap">
							<Text fw={600} truncate>{member.name}</Text>
							{isSelf && <Badge variant="outline" color="gray" size="xs">you</Badge>}
						</Group>
						<Text size="sm" c="dimmed" truncate>{member.email}</Text>
					</Stack>
					<Badge color={member.role === "owner" ? "myColor" : "gray"} variant="light">
						{member.role === "owner" ? "Owner" : "Member"}
					</Badge>
					{isOther && <Menu position="bottom-end" withinPortal>
						<Menu.Target>
							<ActionIcon variant="subtle" color="gray" aria-label={`Actions for ${member.name}`}>
								<IconDots size={16} stroke={1.5} />
							</ActionIcon>
						</Menu.Target>
						<Menu.Dropdown>
							<Menu.Item onClick={() => openConfirmTransferModal({ userId: member.user_id, name: member.name, familyName })}>
								Make owner
							</Menu.Item>
							<Menu.Item color="red" onClick={() => openConfirmRemoveMemberModal({ userId: member.user_id, name: member.name, familyName })}>
								Remove from family
							</Menu.Item>
						</Menu.Dropdown>
					</Menu>}
				</div>;
			})}
		</div>
	</section>;
}

import { Button, Stack, Text } from "@mantine/core";
import { openConfirmDeleteModal, openConfirmLeaveModal } from "../modals/FamilyModals";
import classes from "./FamilyCards.module.css";

/** Members can leave. Leaving keeps the copy of the family's recipes on this device. */
export function LeaveCard({ familyName, selfId }: { familyName: string; selfId?: string }) {
	return <section className={classes.panel}>
		<Stack gap={4}>
			<Text fw={600} size="lg">Leave family</Text>
			<Text size="sm" c="dimmed">
				Your copy of the family's recipes stays on this device as private data. You can rejoin with a new invite code.
			</Text>
		</Stack>
		<div>
			<Button
				variant="outline"
				color="red"
				disabled={!selfId}
				onClick={() => selfId && openConfirmLeaveModal({ userId: selfId, familyName })}
			>
				Leave family
			</Button>
		</div>
	</section>;
}

/** Owners cannot leave. Deleting is the owner's way out, and it asks for the family name first. */
export function DangerZoneCard({ familyName }: { familyName: string }) {
	return <section className={`${classes.panel} ${classes.danger}`}>
		<Stack gap={4}>
			<Text fw={600} size="lg" c="red.4">Danger zone</Text>
			<Text size="sm" c="dimmed">
				Owners can't leave the family. Make someone else the owner first, or delete the family for everyone.
			</Text>
		</Stack>
		<div>
			<Button color="red" onClick={() => openConfirmDeleteModal({ familyName })}>Delete family</Button>
		</div>
	</section>;
}

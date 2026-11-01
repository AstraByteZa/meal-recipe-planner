import { AddMealModal } from "./AddMealModal";
import { ManageTracksModal } from "./ManageTracksModal";
import { SyncSettingsModal } from "../../sync/modals/SyncSettingsModal";
import {
	ConfirmDeleteModal,
	ConfirmLeaveModal,
	ConfirmRemoveMemberModal,
	ConfirmTransferModal,
	RenameFamilyModal,
} from "../../family/modals/FamilyModals";

/** Every planner modal, by name. Pass to `ModalsProvider`. */
export const plannerModals = {
	addMeal: AddMealModal,
	manageTracks: ManageTracksModal,
	syncSettings: SyncSettingsModal,
	renameFamily: RenameFamilyModal,
	confirmRemoveMember: ConfirmRemoveMemberModal,
	confirmTransfer: ConfirmTransferModal,
	confirmDelete: ConfirmDeleteModal,
	confirmLeave: ConfirmLeaveModal,
};

declare module "@mantine/modals" {
	export interface MantineModalsOverride {
		modals: typeof plannerModals;
	}
}

import { AddMealModal } from "./AddMealModal";
import { ManageTracksModal } from "./ManageTracksModal";
import { SyncSettingsModal } from "../../sync/modals/SyncSettingsModal";

/** Every planner modal, by name. Pass to `ModalsProvider`. */
export const plannerModals = {
	addMeal: AddMealModal,
	manageTracks: ManageTracksModal,
	syncSettings: SyncSettingsModal,
};

declare module "@mantine/modals" {
	export interface MantineModalsOverride {
		modals: typeof plannerModals;
	}
}

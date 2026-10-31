import { AddMealModal } from "./AddMealModal";
import { ManageTracksModal } from "./ManageTracksModal";

/** Every planner modal, by name. Pass to `ModalsProvider`. */
export const plannerModals = {
	addMeal: AddMealModal,
	manageTracks: ManageTracksModal,
};

declare module "@mantine/modals" {
	export interface MantineModalsOverride {
		modals: typeof plannerModals;
	}
}

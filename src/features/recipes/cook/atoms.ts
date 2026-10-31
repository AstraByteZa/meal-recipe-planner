import { atom } from "jotai";
import type { CookTimer } from "./timers";

// Cook session state lives only in the browser. Each visit to cook mode resets it.
/** Indexes of the ingredients ticked off as used. */
export const checkedIngredientsAtom = atom<number[]>([]);
/** Indexes of the steps marked done. */
export const doneStepsAtom = atom<number[]>([]);
/** Timers the user has started, paused or reset, by timer id. Timers keep running across step changes. */
export const timersAtom = atom<Record<string, CookTimer>>({});

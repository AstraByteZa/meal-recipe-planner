import { atom } from "jotai";
import { atomWithQuery } from "jotai-tanstack-query";
import { configQueryOptions } from "./queries";
import { appConfigDefaults, type AppConfig } from "./schema";

/** The query behind the config. Components read `appConfigAtom` instead. */
export const configQueryAtom = atomWithQuery(() => configQueryOptions);

/** The current config, or the defaults until the query has data. */
export const appConfigAtom = atom<AppConfig>((get) => get(configQueryAtom).data ?? appConfigDefaults);

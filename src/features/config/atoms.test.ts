import { describe, expect, it, vi } from "vitest";
import { QueryClient } from "@tanstack/react-query";
import { invoke } from "@tauri-apps/api/core";
import { createStore } from "jotai";
import { queryClientAtom } from "jotai-tanstack-query";
import { appConfigAtom } from "./atoms";

// An in-memory stand-in for the config table in the Rust backend.
const backend = vi.hoisted(() => ({ stored: {} as Record<string, unknown> }));

vi.mock("@tauri-apps/api/core", () => ({
	invoke: vi.fn(async (command: string, args?: { key: string; value: unknown }) => {
		if (command === "config_list") return { ...backend.stored };
		if (command === "config_set" && args) {
			backend.stored[args.key] = args.value;
			return;
		}
		throw new Error(`unexpected command ${command}`);
	}),
}));

describe("appConfigAtom", () => {
	it("shows a saved setting once the config query is invalidated", async () => {
		backend.stored = { "cook-keep-awake": true };
		const client = new QueryClient();
		const store = createStore();
		store.set(queryClientAtom, client);
		const unsubscribe = store.sub(appConfigAtom, () => {});

		await vi.waitFor(() => expect(client.getQueryData(["config"])).toBeDefined());
		expect(store.get(appConfigAtom)["cook-keep-awake"]).toBe(true);

		await invoke("config_set", { key: "cook-keep-awake", value: false });
		await client.invalidateQueries({ queryKey: ["config"] });

		await vi.waitFor(() => expect(store.get(appConfigAtom)["cook-keep-awake"]).toBe(false));
		unsubscribe();
	});
});

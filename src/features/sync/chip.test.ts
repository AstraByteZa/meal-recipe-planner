import { describe, expect, it } from "vitest";
import { chipState } from "./chip";
import type { SyncStatus } from "./schema";

const now = new Date("2026-11-01T12:00:00.000Z");

const signedIn: SyncStatus = {
	mode: "signed_in",
	signedIn: true,
	serverUrl: "https://sync.example.com",
	defaultServerUrl: null,
	user: { name: "Alex", email: "alex@example.com" },
	family: null,
	pending: 0,
	syncing: false,
	lastSyncedAt: "2026-11-01T11:58:00.000Z",
	lastError: null,
	lastErrorKind: null,
	notice: null,
};

const with_ = (changes: Partial<SyncStatus>): SyncStatus => ({ ...signedIn, ...changes });

describe("chipState", () => {
	it("is off for a device that is local only", () => {
		const status = with_({ mode: "local", signedIn: false, user: null });
		expect(chipState(status, now)).toEqual({ state: "off", label: "Sync off", detail: "Sign in to sync" });
	});

	it("is off for a linked device with no session", () => {
		const status = with_({ mode: "signed_out", signedIn: false, user: null });
		expect(chipState(status, now).state).toBe("off");
	});

	it("asks for a sign-in again when the session ended", () => {
		const status = with_({ lastError: "token expired", lastErrorKind: "unauthorized" });
		expect(chipState(status, now)).toMatchObject({ state: "error", label: "Sign in again" });
	});

	it("shows a failed sync, and says the server is unreachable when that is the cause", () => {
		const status = with_({ lastError: "connection refused", lastErrorKind: "unreachable" });
		expect(chipState(status, now)).toEqual({ state: "error", label: "Sync failed", detail: "Server unreachable. Retry" });
	});

	it("shows syncing with the number of changes being sent", () => {
		expect(chipState(with_({ syncing: true, pending: 3 }), now)).toEqual({
			state: "busy",
			label: "Syncing…",
			detail: "Sending 3 changes",
		});
	});

	it("shows queued changes while waiting to upload", () => {
		expect(chipState(with_({ pending: 5 }), now)).toEqual({
			state: "waiting",
			label: "5 changes waiting",
			detail: "Will sync when online",
		});
		expect(chipState(with_({ pending: 1 }), now).label).toBe("1 change waiting");
	});

	it("shows synced with the time of the last sync", () => {
		expect(chipState(signedIn, now)).toEqual({ state: "ok", label: "Synced", detail: "Last synced 2 minutes ago" });
	});

	it("says not synced yet before the first sync", () => {
		expect(chipState(with_({ lastSyncedAt: null }), now).detail).toBe("Not synced yet");
	});
});

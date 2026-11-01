import { describe, expect, it } from "vitest";
import { accountFormSchema, familyDetailsSchema, familyJoinSchema, invitePreviewSchema, syncStatusSchema } from "./schema";

const account = { serverUrl: "https://sync.example.com", email: "cook@example.com", password: "correct horse" };

describe("syncStatusSchema", () => {
	it("accepts the status the Rust side returns", () => {
		const status = {
			mode: "signed_in",
			signedIn: true,
			serverUrl: "https://sync.example.com",
			defaultServerUrl: null,
			user: { name: "Roelof", email: "cook@example.com" },
			family: { id: "f1", name: "Home", role: "owner" },
			pending: 3,
			syncing: false,
			lastSyncedAt: "2026-10-31T10:00:00.000Z",
			lastError: null,
			lastErrorKind: null,
			notice: null,
		};
		expect(syncStatusSchema.parse(status)).toEqual(status);
	});

	it("accepts a removal notice", () => {
		const status = {
			mode: "signed_in",
			signedIn: true,
			serverUrl: "https://sync.example.com",
			defaultServerUrl: null,
			user: { name: "Roelof", email: "cook@example.com" },
			family: null,
			pending: 0,
			syncing: false,
			lastSyncedAt: null,
			lastError: null,
			lastErrorKind: null,
			notice: { kind: "removed", familyName: "The Morgans" },
		};
		expect(syncStatusSchema.parse(status).notice?.familyName).toBe("The Morgans");
	});

	it("accepts a device that has never signed in", () => {
		const status = {
			mode: "local",
			signedIn: false,
			serverUrl: null,
			defaultServerUrl: "https://sync.example.com",
			user: null,
			family: null,
			pending: 0,
			syncing: false,
			lastSyncedAt: null,
			lastError: null,
			lastErrorKind: null,
			notice: null,
		};
		expect(syncStatusSchema.parse(status)).toEqual(status);
	});

	it("rejects an unknown account mode", () => {
		const status = { mode: "offline", signedIn: false, serverUrl: null, defaultServerUrl: null, user: null, family: null, pending: 0, syncing: false, lastSyncedAt: null, lastError: null, lastErrorKind: null, notice: null };
		expect(syncStatusSchema.safeParse(status).success).toBe(false);
	});

	it("rejects a negative pending count", () => {
		expect(() =>
			syncStatusSchema.parse({
				signedIn: false,
				serverUrl: null,
				email: null,
				family: null,
				pending: -1,
				syncing: false,
				lastSyncedAt: null,
				lastError: null,
			}),
		).toThrow();
	});
});

describe("accountFormSchema", () => {
	it("accepts a complete account", () => {
		expect(accountFormSchema.safeParse(account).success).toBe(true);
	});

	it("requires an http or https server address", () => {
		expect(accountFormSchema.safeParse({ ...account, serverUrl: "sync.example.com" }).success).toBe(false);
		expect(accountFormSchema.safeParse({ ...account, serverUrl: "ftp://sync.example.com" }).success).toBe(false);
		expect(accountFormSchema.safeParse({ ...account, serverUrl: "http://localhost:8080" }).success).toBe(true);
	});

	it("requires a valid email", () => {
		expect(accountFormSchema.safeParse({ ...account, email: "not-an-email" }).success).toBe(false);
	});

	it("matches the server's password minimum of 8 characters", () => {
		expect(accountFormSchema.safeParse({ ...account, password: "1234567" }).success).toBe(false);
		expect(accountFormSchema.safeParse({ ...account, password: "12345678" }).success).toBe(true);
	});
});

describe("familyJoinSchema", () => {
	it("rejects a blank invite code", () => {
		expect(familyJoinSchema.safeParse({ inviteCode: "   " }).success).toBe(false);
		expect(familyJoinSchema.safeParse({ inviteCode: "" }).success).toBe(false);
	});

	it("needs the full eight characters, with or without the dash", () => {
		expect(familyJoinSchema.safeParse({ inviteCode: "ABC123" }).success).toBe(false);
		expect(familyJoinSchema.safeParse({ inviteCode: "k7m2-qx9p" }).success).toBe(true);
		expect(familyJoinSchema.safeParse({ inviteCode: "K7M2QX9P" }).success).toBe(true);
	});
});

describe("familyDetailsSchema", () => {
	it("accepts the details the owner sees, with snake_case member and invite fields", () => {
		const details = {
			family: { id: "f1", name: "The Morgans", role: "owner" },
			members: [{ user_id: "u1", name: "Alex", email: "alex@example.com", role: "owner", joined_at: "2026-10-01T10:00:00.000Z" }],
			invites: [{ id: "i1", code: "K7M2QX9P", created_at: "2026-10-30T10:00:00.000Z", expires_at: "2026-11-06T10:00:00.000Z" }],
		};
		expect(familyDetailsSchema.parse(details)).toEqual(details);
	});

	it("rejects an unknown role", () => {
		expect(
			familyDetailsSchema.safeParse({ family: { id: "f1", name: "Home", role: "admin" }, members: [], invites: [] }).success,
		).toBe(false);
	});
});

describe("invitePreviewSchema", () => {
	it("accepts the preview the join step shows", () => {
		const preview = { family_name: "The Morgans", invited_by: "Alex", member_count: 3, member_names: ["Alex", "Sam", "Jo"] };
		expect(invitePreviewSchema.parse(preview)).toEqual(preview);
	});
});

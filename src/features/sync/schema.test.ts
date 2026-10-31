import { describe, expect, it } from "vitest";
import { accountFormSchema, familyJoinSchema, syncStatusSchema } from "./schema";

const account = { serverUrl: "https://sync.example.com", email: "cook@example.com", password: "correct horse" };

describe("syncStatusSchema", () => {
	it("accepts the status the Rust side returns", () => {
		const status = {
			signedIn: true,
			serverUrl: "https://sync.example.com",
			email: "cook@example.com",
			family: { id: "f1", name: "Home", inviteCode: "ABC123" },
			pending: 3,
			syncing: false,
			lastSyncedAt: "2026-10-31T10:00:00.000Z",
			lastError: null,
		};
		expect(syncStatusSchema.parse(status)).toEqual(status);
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
		expect(familyJoinSchema.safeParse({ inviteCode: " ABC123 " }).success).toBe(true);
	});
});

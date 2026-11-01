import { describe, expect, it } from "vitest";
import { expiresIn, formatInviteCode, noticeText, normalizeInviteCode, relativeTime } from "./format";

const now = new Date("2026-11-01T12:00:00.000Z");

describe("invite codes", () => {
	it("keeps only code characters, in upper case, and caps the length", () => {
		expect(normalizeInviteCode(" k7m2-qx9p ")).toBe("K7M2QX9P");
		expect(normalizeInviteCode("k7m2qx9p99")).toBe("K7M2QX9P");
		expect(normalizeInviteCode("a!b")).toBe("AB");
	});

	it("shows a full code as XXXX-XXXX", () => {
		expect(formatInviteCode("k7m2qx9p")).toBe("K7M2-QX9P");
	});

	it("groups partial input as it is typed", () => {
		expect(formatInviteCode("k7")).toBe("K7");
		expect(formatInviteCode("k7m2q")).toBe("K7M2-Q");
		expect(formatInviteCode("")).toBe("");
	});

	it("formats a pasted code with the dash already in it", () => {
		expect(formatInviteCode("K7M2-QX9P")).toBe("K7M2-QX9P");
	});
});

describe("expiresIn", () => {
	it("counts whole days when more than two days are left", () => {
		expect(expiresIn("2026-11-06T12:00:00.000Z", now)).toBe("Expires in 5 days");
	});

	it("counts hours when less than two days are left", () => {
		expect(expiresIn("2026-11-01T15:00:00.000Z", now)).toBe("Expires in 3 hours");
		expect(expiresIn("2026-11-01T13:00:00.000Z", now)).toBe("Expires in 1 hour");
	});

	it("says less than an hour when under an hour is left", () => {
		expect(expiresIn("2026-11-01T12:30:00.000Z", now)).toBe("Expires in less than an hour");
	});

	it("reads a past time as expired", () => {
		expect(expiresIn("2026-11-01T11:00:00.000Z", now)).toBe("Expired");
	});
});

describe("relativeTime", () => {
	it("describes how long ago a time was", () => {
		expect(relativeTime("2026-11-01T11:58:00.000Z", now)).toBe("2 minutes ago");
	});
});

describe("noticeText", () => {
	it("says removed by default and deleted for a deleted family", () => {
		expect(noticeText("removed", "The Morgans")).toContain("You were removed from The Morgans");
		expect(noticeText("deleted", "The Morgans")).toBe("The Morgans was deleted by its owner.");
	});
});

import { describe, expect, it } from "vitest";
import { findSelf, initialOf, memberSummary } from "./format";

describe("memberSummary", () => {
	it("pluralises the count and names the role", () => {
		expect(memberSummary(3, "owner")).toBe("3 members · You are the owner");
		expect(memberSummary(1, "member")).toBe("1 member · You are a member");
	});
});

describe("findSelf", () => {
	const members = [
		{ email: "alex@example.com", name: "Alex" },
		{ email: "sam@example.com", name: "Sam" },
	];

	it("finds the member with the signed-in email", () => {
		expect(findSelf(members, "sam@example.com")?.name).toBe("Sam");
	});

	it("returns nothing when signed out or not listed", () => {
		expect(findSelf(members, undefined)).toBeUndefined();
		expect(findSelf(members, "jo@example.com")).toBeUndefined();
	});
});

describe("initialOf", () => {
	it("uses the first letter in upper case", () => {
		expect(initialOf("  sam")).toBe("S");
		expect(initialOf("")).toBe("");
	});
});

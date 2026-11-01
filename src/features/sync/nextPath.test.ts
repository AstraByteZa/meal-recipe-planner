import { describe, expect, it } from "vitest";
import { toNextPath } from "./nextPath";

describe("toNextPath", () => {
	it("keeps in-app paths, including a query string", () => {
		expect(toNextPath("/join")).toBe("/join");
		expect(toNextPath("/onboarding?step=2")).toBe("/onboarding?step=2");
	});

	it("drops anything that could leave the app", () => {
		expect(toNextPath("https://example.com")).toBeUndefined();
		expect(toNextPath("//example.com")).toBeUndefined();
		expect(toNextPath("/\\example.com")).toBeUndefined();
		expect(toNextPath("join")).toBeUndefined();
	});

	it("ignores values that are not strings", () => {
		expect(toNextPath(undefined)).toBeUndefined();
		expect(toNextPath(3)).toBeUndefined();
	});
});

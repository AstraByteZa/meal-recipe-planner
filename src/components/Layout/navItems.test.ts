import { describe, expect, it } from "vitest";
import { navItems } from "./navItems";

describe("navItems", () => {
	it("hides Family when signed out", () => {
		expect(navItems(false).map((item) => item.to)).toEqual(["/", "/recipes"]);
	});

	it("adds Family last when signed in", () => {
		const items = navItems(true);
		expect(items.map((item) => item.to)).toEqual(["/", "/recipes", "/family"]);
		expect(items[2].label).toBe("Family");
	});
});

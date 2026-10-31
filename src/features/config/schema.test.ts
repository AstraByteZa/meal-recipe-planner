import { describe, expect, it } from "vitest";
import { appConfigDefaults, appConfigSchema } from "./schema";

describe("appConfigSchema", () => {
	it("uses the defaults when nothing is stored", () => {
		expect(appConfigSchema.parse({})).toEqual({
			"cook-keep-awake": true,
			"cook-text-scale": "md",
			"cook-timer-sound": true,
		});
		expect(appConfigDefaults["cook-text-scale"]).toBe("md");
	});

	it("keeps valid stored values", () => {
		const parsed = appConfigSchema.parse({ "cook-keep-awake": false, "cook-text-scale": "lg" });
		expect(parsed["cook-keep-awake"]).toBe(false);
		expect(parsed["cook-text-scale"]).toBe("lg");
	});

	it("falls back to the default for a value of the wrong type", () => {
		const parsed = appConfigSchema.parse({ "cook-keep-awake": "yes", "cook-text-scale": "huge" });
		expect(parsed["cook-keep-awake"]).toBe(true);
		expect(parsed["cook-text-scale"]).toBe("md");
	});

	it("drops keys it does not know", () => {
		const parsed = appConfigSchema.parse({ "some-future-key": 1 });
		expect(parsed).not.toHaveProperty("some-future-key");
	});
});

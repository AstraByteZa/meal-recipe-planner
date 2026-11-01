import { describe, expect, it } from "vitest";
import { customServerUrl, startsOpen } from "./serverUrl";

const BUILT_IN = "https://sync.example.com";

describe("customServerUrl", () => {
	it("is blank when the device uses the built-in server", () => {
		expect(customServerUrl({ serverUrl: BUILT_IN, defaultServerUrl: BUILT_IN })).toBe("");
	});

	it("shows a self-hosted address that differs from the built-in server", () => {
		expect(customServerUrl({ serverUrl: "http://10.0.0.5:8080", defaultServerUrl: BUILT_IN })).toBe("http://10.0.0.5:8080");
	});

	it("is blank when there is no address at all", () => {
		expect(customServerUrl({ serverUrl: null, defaultServerUrl: null })).toBe("");
	});

	it("shows a stored address when this build has no built-in server", () => {
		expect(customServerUrl({ serverUrl: "http://10.0.0.5:8080", defaultServerUrl: null })).toBe("http://10.0.0.5:8080");
	});
});

describe("startsOpen", () => {
	it("is open only when there is no built-in server and no stored address", () => {
		expect(startsOpen({ serverUrl: null, defaultServerUrl: null })).toBe(true);
		expect(startsOpen({ serverUrl: BUILT_IN, defaultServerUrl: BUILT_IN })).toBe(false);
		expect(startsOpen({ serverUrl: "http://10.0.0.5:8080", defaultServerUrl: null })).toBe(false);
	});
});

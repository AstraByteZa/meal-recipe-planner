import type { SyncStatus } from "./schema";

type ServerFields = Pick<SyncStatus, "serverUrl" | "defaultServerUrl">;

/**
 * The address to pre-fill in Advanced Options. Only a custom address is shown: when the device uses
 * the built-in server, the field stays blank, so a blank field always means the built-in server.
 */
export function customServerUrl({ serverUrl, defaultServerUrl }: ServerFields): string {
	return serverUrl && serverUrl !== defaultServerUrl ? serverUrl : "";
}

/**
 * Whether Advanced Options starts open. It does when there is no built-in server and no stored
 * address, because the user has to enter one before signing in can work.
 */
export function startsOpen({ serverUrl, defaultServerUrl }: ServerFields): boolean {
	return serverUrl === null && defaultServerUrl === null;
}

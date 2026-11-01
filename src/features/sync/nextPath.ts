/**
 * The in-app page to go to after signing in. Only paths on this app are allowed, so a link can
 * never send the user to another site. Returns `undefined` for anything else.
 */
export function toNextPath(value: unknown): string | undefined {
	if (typeof value !== "string") return undefined;
	if (!value.startsWith("/") || value.startsWith("//") || value.startsWith("/\\")) return undefined;
	return value;
}

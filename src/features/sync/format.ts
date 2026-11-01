import dayjs from "../../lib/dayjs";

const CODE_LENGTH = 8;

/** Keeps only the characters an invite code can contain, in upper case, without the dash. */
export function normalizeInviteCode(raw: string): string {
	return raw.toUpperCase().replace(/[^A-Z0-9]/g, "").slice(0, CODE_LENGTH);
}

/** Shows a code as `XXXX-XXXX`. Partial input is grouped as it is typed. */
export function formatInviteCode(raw: string): string {
	const code = normalizeInviteCode(raw);
	return code.length > 4 ? `${code.slice(0, 4)}-${code.slice(4)}` : code;
}

/**
 * How long an invite code has left, for example "Expires in 5 days". Past times read as expired.
 * `now` is a parameter so the function stays pure.
 */
export function expiresIn(expiresAt: string, now: Date): string {
	const end = dayjs(expiresAt);
	const at = dayjs(now);
	if (!end.isAfter(at)) return "Expired";

	const hours = end.diff(at, "hour");
	if (hours < 1) return "Expires in less than an hour";
	if (hours < 48) return `Expires in ${hours} ${hours === 1 ? "hour" : "hours"}`;
	const days = end.diff(at, "day");
	return `Expires in ${days} days`;
}

/** Relative time such as "2 minutes ago", for when the device last synced. */
export function relativeTime(iso: string, now: Date): string {
	return dayjs(iso).from(dayjs(now));
}

/** The text on the removed or deleted banner. */
export function noticeText(kind: string, familyName: string): string {
	if (kind === "deleted") return `${familyName} was deleted by its owner.`;
	return `You were removed from ${familyName}. Your recipes are kept on this device as private data.`;
}

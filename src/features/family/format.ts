import type { FamilyRole } from "../sync/schema";

/** "3 members · You are the owner". The count is singular for one member. */
export function memberSummary(count: number, role: FamilyRole): string {
	const members = count === 1 ? "1 member" : `${count} members`;
	return `${members} · ${role === "owner" ? "You are the owner" : "You are a member"}`;
}

/** The member row for the signed-in account. The email is the only identity the status carries. */
export function findSelf<T extends { email: string }>(members: T[], email: string | undefined): T | undefined {
	if (!email) return undefined;
	return members.find((member) => member.email === email);
}

/** Initial for an avatar, from a name or an email. */
export function initialOf(name: string): string {
	return name.trim().slice(0, 1).toUpperCase();
}

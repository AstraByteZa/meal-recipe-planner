import { z } from "zod";
import { normalizeInviteCode } from "./format";

/** Sync state from the `sync_*` and `family_*` commands. Mirrors `SyncStatus` in src-tauri. */
export const syncStatusSchema = z.object({
	/** `local`: never signed in. `signed_out`: linked, no token. `signed_in`: syncing. */
	mode: z.enum(["local", "signed_out", "signed_in"]),
	signedIn: z.boolean(),
	serverUrl: z.string().nullable(),
	/** The address built into the app. Pre-fills the server field. */
	defaultServerUrl: z.string().nullable(),
	/** Only while signed in. `name` is the display name, or the email's local part. */
	user: z.object({ name: z.string(), email: z.string() }).nullable(),
	family: z
		.object({ id: z.string(), name: z.string(), role: z.enum(["owner", "member"]) })
		.nullable(),
	/** Always 0 unless signed in. */
	pending: z.number().int().nonnegative(),
	syncing: z.boolean(),
	lastSyncedAt: z.string().nullable(),
	lastError: z.string().nullable(),
	lastErrorKind: z.enum(["unauthorized", "unreachable", "other"]).nullable(),
	/** Set after this device was removed from a family, until the user dismisses it. */
	notice: z.object({ kind: z.string(), familyName: z.string() }).nullable(),
});

export type SyncStatus = z.infer<typeof syncStatusSchema>;

export const familyRoleSchema = z.enum(["owner", "member"]);
export type FamilyRole = z.infer<typeof familyRoleSchema>;

/** Family details use the server's field names as they are, without camelCase. */
export const familyDetailsSchema = z.object({
	family: z.object({ id: z.string(), name: z.string(), role: familyRoleSchema }),
	members: z.array(
		z.object({
			user_id: z.string(),
			name: z.string(),
			email: z.string(),
			role: familyRoleSchema,
			joined_at: z.string(),
		}),
	),
	/** Empty unless the caller is the owner. */
	invites: z.array(
		z.object({ id: z.string(), code: z.string(), created_at: z.string(), expires_at: z.string() }),
	),
});

export type FamilyDetails = z.infer<typeof familyDetailsSchema>;

/** A new invite code, returned when the owner creates one. */
export const familyInviteSchema = z.object({
	id: z.string(),
	code: z.string(),
	created_at: z.string(),
	expires_at: z.string(),
});

export type FamilyInvite = z.infer<typeof familyInviteSchema>;

/** What an invite code leads to. Checking a code does not use it. */
export const invitePreviewSchema = z.object({
	family_name: z.string(),
	invited_by: z.string(),
	member_count: z.number().int().nonnegative(),
	member_names: z.array(z.string()),
});

export type InvitePreview = z.infer<typeof invitePreviewSchema>;

function isHttpUrl(value: string) {
	try {
		const { protocol } = new URL(value);
		return protocol === "http:" || protocol === "https:";
	} catch {
		return false;
	}
}

/** Sign in or register. The password rule matches the server's minimum. */
export const accountFormSchema = z.object({
	/** Blank means the built-in server. A typed address must be a full http:// or https:// URL. */
	serverUrl: z
		.string()
		.trim()
		.refine((value) => value === "" || isHttpUrl(value), "Enter a full http:// or https:// address"),
	email: z.string().trim().pipe(z.email("Enter a valid email address")),
	password: z
		.string()
		.min(8, "Use at least 8 characters")
		.max(1024, "Use at most 1024 characters"),
});

export const familyNameSchema = z.object({
	name: z.string().trim().min(1, "Give the family a name"),
});

export const familyJoinSchema = z.object({
	inviteCode: z.string().superRefine((value, ctx) => {
		const length = normalizeInviteCode(value).length;
		if (length === 0) ctx.addIssue({ code: "custom", message: "Enter the invite code" });
		else if (length !== 8) ctx.addIssue({ code: "custom", message: "Invite codes have 8 characters" });
	}),
});

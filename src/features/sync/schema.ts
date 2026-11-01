import { z } from "zod";

/** Sync state from the `sync_*` and `family_*` commands. Mirrors `SyncStatus` in src-tauri. */
export const syncStatusSchema = z.object({
	signedIn: z.boolean(),
	serverUrl: z.string().nullable(),
	email: z.string().nullable(),
	family: z
		.object({ id: z.string(), name: z.string() })
		.nullable(),
	pending: z.number().int().nonnegative(),
	syncing: z.boolean(),
	lastSyncedAt: z.string().nullable(),
	lastError: z.string().nullable(),
});

export type SyncStatus = z.infer<typeof syncStatusSchema>;

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
	serverUrl: z.string().trim().refine(isHttpUrl, "Enter a full http:// or https:// address"),
	email: z.string().trim().pipe(z.email("Enter a valid email address")),
	password: z
		.string()
		.min(8, "Use at least 8 characters")
		.max(1024, "Use at most 1024 characters"),
});

export const familyCreateSchema = z.object({
	name: z.string().trim().min(1, "Give the family a name"),
});

export const familyJoinSchema = z.object({
	inviteCode: z.string().trim().min(1, "Enter the invite code"),
});

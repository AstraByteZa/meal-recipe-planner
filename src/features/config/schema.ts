import { z } from "zod";

/**
 * App configuration, stored in the `config` table. Keys are kebab-case to match the database.
 * Each field falls back to its default when the stored value is missing or the wrong type,
 * so one bad value cannot break the app.
 */
export const appConfigSchema = z.object({
	"cook-keep-awake": z.boolean().catch(true),
	"cook-text-scale": z.enum(["sm", "md", "lg"]).catch("md"),
	"cook-timer-sound": z.boolean().catch(true),
});

export type AppConfig = z.infer<typeof appConfigSchema>;
export type AppConfigKey = keyof AppConfig;

/** Values used before the config loads, and for any field that fails to parse. */
export const appConfigDefaults: AppConfig = appConfigSchema.parse({});

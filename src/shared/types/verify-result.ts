import { z } from "zod";

/**
 * These schemas are the single point of contact with the Rust backend's
 * serde output. `invoke<T>()` from `@tauri-apps/api` is a compile-time-only
 * type assertion — it does not check anything at runtime, so if the Rust
 * struct shape ever drifts from these TS types (a renamed field, a dropped
 * `camelCase` rename, a widened enum), the frontend would otherwise get
 * silently wrong data instead of a caught error. Parsing every command
 * response through these schemas (see shared/lib/tauri.ts) converts that
 * drift into a thrown, debuggable error at the call site.
 */

export const verdictSchema = z.enum([
	"Valid",
	"Invalid",
	"RiskyCatchAll",
	"Unknown",
]);
export type Verdict = z.infer<typeof verdictSchema>;

export const verifyResultSchema = z.object({
	id: z.string(),
	email: z.string(),
	syntaxValid: z.boolean(),
	mxFound: z.boolean(),
	mxRecords: z.array(z.string()),
	catchAll: z.boolean().nullable(),
	smtpCode: z.number().int().nullable(),
	smtpMessage: z.string(),
	error: z.string().nullable(),
	verdict: verdictSchema,
	checkedAt: z.string(),
	durationMs: z.number().int(),
});
export type VerifyResult = z.infer<typeof verifyResultSchema>;

export const batchSummarySchema = z.object({
	total: z.number().int(),
	valid: z.number().int(),
	invalid: z.number().int(),
	unknown: z.number().int(),
	riskyCatchAll: z.number().int(),
});
export type BatchSummary = z.infer<typeof batchSummarySchema>;

export const batchRunResultSchema = z.tuple([
	z.array(verifyResultSchema),
	batchSummarySchema,
]);

export const dashboardStatsSchema = z.object({
	totalVerifiedAllTime: z.number().int(),
	overallValidRate: z.number(),
	catchAllDomainCount: z.number().int(),
	verifiedToday: z.number().int(),
});
export type DashboardStats = z.infer<typeof dashboardStatsSchema>;

export const networkHealthSchema = z.object({
	port25Reachable: z.boolean(),
	checkedAt: z.string(),
	detail: z.string().nullable(),
});
export type NetworkHealth = z.infer<typeof networkHealthSchema>;

export const dnsResolverSchema = z.enum(["system", "cloudflare", "google"]);
export type DnsResolver = z.infer<typeof dnsResolverSchema>;

export const settingsSchema = z.object({
	heloDomain: z.string(),
	smtpTimeoutSeconds: z.number().int(),
	domainCooldownSeconds: z.number().int(),
	maxConcurrentDomains: z.number().int(),
	dnsResolver: dnsResolverSchema,
});
export type Settings = z.infer<typeof settingsSchema>;

export const batchProgressSchema = z.tuple([
	z.number().int(),
	z.number().int(),
]);

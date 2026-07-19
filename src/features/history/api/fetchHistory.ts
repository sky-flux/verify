import { z } from "zod";
import { callCommand } from "@/shared/lib/tauri";
import {
	type VerifyResult,
	verifyResultSchema,
} from "@/shared/types/verify-result";

export function fetchHistory(params: {
	domainFilter?: string;
	emailSearch?: string;
	limit?: number;
	offset?: number;
}): Promise<VerifyResult[]> {
	return callCommand("fetch_history", z.array(verifyResultSchema), params);
}

export function countHistory(params: {
	domainFilter?: string;
	emailSearch?: string;
}): Promise<number> {
	return callCommand("count_history", z.number().int(), params);
}

export function fetchDistinctDomains(): Promise<string[]> {
	return callCommand("fetch_distinct_domains", z.array(z.string()));
}

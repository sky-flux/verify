import { z } from "zod";
import { callCommand } from "@/shared/lib/tauri";
import {
	type BatchSummary,
	batchRunResultSchema,
	type VerifyResult,
} from "@/shared/types/verify-result";

export async function verifyBatchEmails(
	emails: string[],
): Promise<{ results: VerifyResult[]; summary: BatchSummary }> {
	const [results, summary] = await callCommand(
		"verify_batch_emails",
		batchRunResultSchema,
		{
			emails,
		},
	);
	return { results, summary };
}

export function cancelBatchVerification(): Promise<null> {
	return callCommand("cancel_batch_verification", z.null());
}

export function exportResultsToCsv(
	results: VerifyResult[],
	filePath: string,
): Promise<null> {
	return callCommand("export_results_to_csv", z.null(), { results, filePath });
}

/** Parses imported CSV/TXT file content into email addresses — the parsing
 * itself always happens in Rust, per the project's core principle. */
export function parseImportedEmails(content: string): Promise<string[]> {
	return callCommand("parse_imported_emails", z.array(z.string()), {
		content,
	});
}

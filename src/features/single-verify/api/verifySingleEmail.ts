import { callCommand } from "@/shared/lib/tauri";
import {
	type VerifyResult,
	verifyResultSchema,
} from "@/shared/types/verify-result";

/**
 * Pass `existingId` when re-verifying a row already shown in History — the
 * backend reuses that id so the row is updated in place instead of a
 * duplicate entry being inserted for the same address.
 */
export function verifySingleEmail(
	email: string,
	existingId?: string,
): Promise<VerifyResult> {
	return callCommand("verify_single_email", verifyResultSchema, {
		email,
		existingId,
	});
}

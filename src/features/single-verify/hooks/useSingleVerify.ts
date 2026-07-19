import { useState } from "react";
import { AppCommandError } from "@/shared/lib/tauri";
import type { VerifyResult } from "@/shared/types/verify-result";
import { verifySingleEmail } from "../api/verifySingleEmail";

type Status = "idle" | "loading" | "result" | "error";

/** `initialResult` seeds the hook straight into the "result" state — used
 * to hand off an already-fetched result (e.g. from Dashboard's quick-verify
 * card) without a redundant re-probe. Only read on first render. */
export function useSingleVerify(initialResult?: VerifyResult | null) {
	const [status, setStatus] = useState<Status>(
		initialResult ? "result" : "idle",
	);
	const [result, setResult] = useState<VerifyResult | null>(
		initialResult ?? null,
	);
	const [error, setError] = useState<string | null>(null);
	const [lastEmail, setLastEmail] = useState(initialResult?.email ?? "");

	// No useCallback here — the React Compiler (see vite.config.ts) infers
	// memoization automatically, so wrapping these manually is redundant.
	const verify = async (email: string) => {
		setStatus("loading");
		setError(null);
		setLastEmail(email);
		try {
			const verifyResult = await verifySingleEmail(email);
			setResult(verifyResult);
			setStatus("result");
		} catch (e) {
			setError(e instanceof AppCommandError ? e.message : String(e));
			setStatus("error");
		}
	};

	const reverify = () => {
		if (lastEmail) void verify(lastEmail);
	};

	const reset = () => {
		setStatus("idle");
		setResult(null);
		setError(null);
		setLastEmail("");
	};

	return { status, result, error, verify, reverify, reset };
}

import { create } from "zustand";
import type { VerifyResult } from "@/shared/types/verify-result";

interface HandoffState {
	result: VerifyResult | null;
	setResult: (result: VerifyResult) => void;
	consumeResult: () => VerifyResult | null;
}

/**
 * Carries a just-verified result from Dashboard's quick-verify card over to
 * the full /single page so "查看完整详情" doesn't force a redundant
 * re-verification of the same address (an extra live SMTP probe) just to
 * show the same result again. Read-once: consumeResult clears it, so a
 * stale result never silently reappears on a later, unrelated visit to
 * /single.
 */
export const useHandoffStore = create<HandoffState>((set, get) => ({
	result: null,
	setResult: (result) => set({ result }),
	consumeResult: () => {
		const result = get().result;
		if (result) set({ result: null });
		return result;
	},
}));

import { create } from "zustand";
import { onEvent } from "@/shared/lib/tauri";
import {
	type BatchSummary,
	batchProgressSchema,
	type VerifyResult,
} from "@/shared/types/verify-result";
import {
	cancelBatchVerification,
	verifyBatchEmails,
} from "../api/verifyBatchEmails";

type BatchStatus = "idle" | "running" | "cancelling" | "done";

interface BatchState {
	batchStatus: BatchStatus;
	batchProgress: { completed: number; total: number };
	batchResults: VerifyResult[];
	batchSummary: BatchSummary | null;
	batchError: string | null;

	startBatch: (emails: string[]) => Promise<void>;
	cancelBatch: () => Promise<void>;
	resetBatch: () => void;
	updateResult: (updated: VerifyResult) => void;
}

export const useBatchStore = create<BatchState>((set, get) => ({
	batchStatus: "idle",
	batchProgress: { completed: 0, total: 0 },
	batchResults: [],
	batchSummary: null,
	batchError: null,

	startBatch: async (emails) => {
		set({
			batchStatus: "running",
			batchProgress: { completed: 0, total: emails.length },
			batchResults: [],
			batchSummary: null,
			batchError: null,
		});

		const unlisten = await onEvent(
			"verify-progress",
			batchProgressSchema,
			([completed, total]) => {
				set({ batchProgress: { completed, total } });
			},
		);

		try {
			const { results, summary } = await verifyBatchEmails(emails);
			set({
				batchStatus: "done",
				batchResults: results,
				batchSummary: summary,
			});
		} catch (e) {
			// Without this, a rejected command left batchStatus stuck on
			// "running" forever with no way back to the input screen —
			// violates "any wait on a Rust command must never be silent".
			set({
				batchStatus: "idle",
				batchError: e instanceof Error ? e.message : String(e),
			});
		} finally {
			unlisten();
		}
	},

	cancelBatch: async () => {
		if (get().batchStatus !== "running") return;
		set({ batchStatus: "cancelling" });
		await cancelBatchVerification();
	},

	resetBatch: () => {
		set({
			batchStatus: "idle",
			batchProgress: { completed: 0, total: 0 },
			batchResults: [],
			batchSummary: null,
			batchError: null,
		});
	},

	updateResult: (updated) => {
		set((state) => ({
			batchResults: state.batchResults.map((r) =>
				r.id === updated.id ? updated : r,
			),
		}));
	},
}));

import { useEffect, useState } from "react";
import { fetchHistory } from "@/features/history/api/fetchHistory";
import type {
	DashboardStats,
	VerifyResult,
} from "@/shared/types/verify-result";
import { getDashboardStats } from "../api/getDashboardStats";

export function useDashboardStats() {
	const [stats, setStats] = useState<DashboardStats | null>(null);
	const [recent, setRecent] = useState<VerifyResult[]>([]);
	const [loading, setLoading] = useState(true);

	const refresh = async () => {
		setLoading(true);
		const [statsResult, recentResult] = await Promise.all([
			getDashboardStats(),
			fetchHistory({ limit: 10 }),
		]);
		setStats(statsResult);
		setRecent(recentResult);
		setLoading(false);
	};

	// `refresh` closes over no reactive state/props (only setters, which are
	// always stable), so this only needs to run once on mount — depending on
	// `refresh`'s function identity instead (as before) caused an infinite
	// reload loop in `tauri dev`'s live transform, where the React Compiler
	// pass doesn't stabilize it the way a production `vite build` does.
	// biome-ignore lint/correctness/useExhaustiveDependencies: intentionally run-once-on-mount — see comment above.
	useEffect(() => {
		void refresh();
	}, []);

	return { stats, recent, loading, refresh };
}

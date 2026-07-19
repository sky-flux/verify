import { useDashboardStats } from "../hooks/useDashboardStats";
import { QuickVerifyCard } from "./QuickVerifyCard";
import { RecentActivityTable } from "./RecentActivityTable";
import { StatsCards } from "./StatsCards";

export function DashboardPage() {
	const { stats, recent, loading, refresh } = useDashboardStats();

	return (
		<div className="flex flex-col gap-6">
			<StatsCards stats={stats} loading={loading} />
			<QuickVerifyCard onVerified={refresh} />
			<RecentActivityTable results={recent} onRowUpdated={refresh} />
		</div>
	);
}

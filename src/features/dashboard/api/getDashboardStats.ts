import { callCommand } from "@/shared/lib/tauri";
import {
	type DashboardStats,
	dashboardStatsSchema,
	type NetworkHealth,
	networkHealthSchema,
} from "@/shared/types/verify-result";

export function getDashboardStats(): Promise<DashboardStats> {
	return callCommand("get_dashboard_stats", dashboardStatsSchema);
}

export function checkNetworkHealth(): Promise<NetworkHealth> {
	return callCommand("check_network_health", networkHealthSchema);
}

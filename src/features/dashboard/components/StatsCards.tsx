import {
	Card,
	CardContent,
	CardHeader,
	CardTitle,
} from "@/shared/components/ui/card";
import { Skeleton } from "@/shared/components/ui/skeleton";
import {
	Tooltip,
	TooltipContent,
	TooltipTrigger,
} from "@/shared/components/ui/tooltip";
import type { DashboardStats } from "@/shared/types/verify-result";

export function StatsCards({
	stats,
	loading,
}: {
	stats: DashboardStats | null;
	loading: boolean;
}) {
	const cards = [
		{
			label: "累计验证数量",
			value: stats ? stats.totalVerifiedAllTime.toLocaleString() : "",
			tooltip: "全部历史验证记录的总数",
		},
		{
			label: "整体有效率",
			value: stats ? `${(stats.overallValidRate * 100).toFixed(1)}%` : "",
			tooltip: "基于全部历史验证记录计算",
		},
		{
			label: "Catch-all 域名数",
			value: stats ? stats.catchAllDomainCount.toLocaleString() : "",
			tooltip: "这些域名的历史结果不可信",
		},
		{
			label: "今日验证数量",
			value: stats ? stats.verifiedToday.toLocaleString() : "",
			tooltip: "当天 0 点以来的验证数",
		},
	];

	return (
		<div className="grid grid-cols-2 gap-4 md:grid-cols-4">
			{cards.map((c) => (
				<Tooltip key={c.label}>
					<TooltipTrigger render={<Card />}>
						<CardHeader className="pb-2">
							<CardTitle className="text-sm font-normal text-muted-foreground">
								{c.label}
							</CardTitle>
						</CardHeader>
						<CardContent>
							{loading ? (
								<Skeleton className="h-8 w-16" />
							) : (
								<div className="text-2xl font-semibold">{c.value}</div>
							)}
						</CardContent>
					</TooltipTrigger>
					<TooltipContent>{c.tooltip}</TooltipContent>
				</Tooltip>
			))}
		</div>
	);
}

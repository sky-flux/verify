import {
	Card,
	CardContent,
	CardHeader,
	CardTitle,
} from "@/shared/components/ui/card";
import type { BatchSummary } from "@/shared/types/verify-result";

function pct(part: number, total: number) {
	return total > 0 ? `${((part / total) * 100).toFixed(1)}%` : "0%";
}

export function BatchSummaryCards({ summary }: { summary: BatchSummary }) {
	const cards = [
		{ label: "有效", value: summary.valid },
		{ label: "无效", value: summary.invalid },
		{ label: "未知", value: summary.unknown },
		{ label: "Catch-all 风险", value: summary.riskyCatchAll },
	];
	return (
		<div className="grid grid-cols-2 gap-4 md:grid-cols-4">
			{cards.map((c) => (
				<Card key={c.label}>
					<CardHeader className="pb-2">
						<CardTitle className="text-sm font-normal text-muted-foreground">
							{c.label}
						</CardTitle>
					</CardHeader>
					<CardContent>
						<div className="text-2xl font-semibold">{c.value}</div>
						<div className="text-xs text-muted-foreground">
							{pct(c.value, summary.total)}
						</div>
					</CardContent>
				</Card>
			))}
		</div>
	);
}

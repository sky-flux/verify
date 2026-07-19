import { Badge } from "@/shared/components/ui/badge";
import { cn } from "@/shared/lib/utils";
import type { Verdict } from "@/shared/types/verify-result";

const STYLES: Record<Verdict, string> = {
	Valid:
		"bg-green-100 text-green-700 hover:bg-green-100 dark:bg-green-950 dark:text-green-400",
	Invalid:
		"bg-red-100 text-red-700 hover:bg-red-100 dark:bg-red-950 dark:text-red-400",
	Unknown:
		"bg-yellow-100 text-yellow-700 hover:bg-yellow-100 dark:bg-yellow-950 dark:text-yellow-400",
	RiskyCatchAll:
		"bg-orange-100 text-orange-700 hover:bg-orange-100 dark:bg-orange-950 dark:text-orange-400",
};

const LABELS: Record<Verdict, string> = {
	Valid: "有效",
	Invalid: "无效",
	Unknown: "未知，建议稍后重试",
	RiskyCatchAll: "Catch-all，结果不可信",
};

export function VerdictBadge({
	verdict,
	className,
}: {
	verdict: Verdict;
	className?: string;
}) {
	return (
		<Badge className={cn(STYLES[verdict], className)}>{LABELS[verdict]}</Badge>
	);
}

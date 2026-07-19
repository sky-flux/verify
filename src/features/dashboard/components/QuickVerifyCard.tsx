import { Link } from "@tanstack/react-router";
import { useEffect } from "react";
import { VerdictBadge } from "@/features/batch-verify/components/VerdictBadge";
import { SingleVerifyForm } from "@/features/single-verify/components/SingleVerifyForm";
import { useSingleVerify } from "@/features/single-verify/hooks/useSingleVerify";
import { useHandoffStore } from "@/features/single-verify/store/handoffStore";
import { Button } from "@/shared/components/ui/button";
import {
	Card,
	CardContent,
	CardHeader,
	CardTitle,
} from "@/shared/components/ui/card";

export function QuickVerifyCard({ onVerified }: { onVerified: () => void }) {
	const { status, result, verify } = useSingleVerify();

	// This card's own useSingleVerify instance never touches the sibling
	// useDashboardStats hook, so a fresh result here wouldn't otherwise be
	// reflected in the stat tiles/recent-activity list until the user left
	// and re-entered the page. Firing onVerified whenever a new result lands
	// keeps the dashboard in sync in real time instead of on next mount.
	//
	// Deliberately NOT depending on `onVerified` here: it's DashboardPage's
	// `refresh` from useDashboardStats, which is a plain (non-memoized)
	// function re-created on every DashboardPage render — including renders
	// triggered by refresh() itself completing. Depending on its identity
	// caused an infinite loop (refresh → re-render → new onVerified
	// reference → effect re-fires → refresh again → ...). Calling a
	// possibly-stale closure of it is safe here since its behavior never
	// changes between renders (it always does the same stats/history fetch).
	// biome-ignore lint/correctness/useExhaustiveDependencies: intentionally omits `onVerified` — see comment above.
	useEffect(() => {
		if (status === "result" && result) onVerified();
	}, [status, result]);

	return (
		<Card>
			<CardHeader className="flex flex-row items-center justify-between">
				<CardTitle>快速验证一个邮箱</CardTitle>
				<Button
					render={<Link to="/batch" />}
					nativeButton={false}
					variant="link"
					size="sm"
				>
					开始批量验证
				</Button>
			</CardHeader>
			<CardContent className="flex flex-col gap-3">
				<SingleVerifyForm
					compact
					loading={status === "loading"}
					onSubmit={verify}
				/>
				{status === "result" && result && (
					<div className="flex items-center gap-2 text-sm">
						<span className="truncate">{result.email}</span>
						<VerdictBadge verdict={result.verdict} />
						<span className="text-muted-foreground">
							SMTP {result.smtpCode ?? "N/A"}
						</span>
						<Link
							to="/single"
							className="ml-auto text-primary underline-offset-4 hover:underline"
							onClick={() => useHandoffStore.getState().setResult(result)}
						>
							查看完整详情
						</Link>
					</div>
				)}
			</CardContent>
		</Card>
	);
}

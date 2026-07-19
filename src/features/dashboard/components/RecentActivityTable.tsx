import { Link } from "@tanstack/react-router";
import { Button } from "@/shared/components/ui/button";
import {
	Card,
	CardContent,
	CardHeader,
	CardTitle,
} from "@/shared/components/ui/card";
import { VerifyResultsTable } from "@/shared/components/verify-results-table";
import type { VerifyResult } from "@/shared/types/verify-result";

export function RecentActivityTable({
	results,
	onRowUpdated,
}: {
	results: VerifyResult[];
	onRowUpdated?: (updated: VerifyResult) => void;
}) {
	return (
		<Card>
			<CardHeader className="flex flex-row items-center justify-between">
				<CardTitle>最近验证</CardTitle>
				<Button
					render={<Link to="/history" />}
					nativeButton={false}
					variant="link"
					size="sm"
				>
					查看全部
				</Button>
			</CardHeader>
			<CardContent>
				{results.length === 0 ? (
					<div className="flex flex-col items-center gap-2 py-8 text-muted-foreground">
						<span>暂无验证记录</span>
						<Button
							render={<Link to="/single" />}
							nativeButton={false}
							variant="outline"
							size="sm"
						>
							去验证第一个邮箱
						</Button>
					</div>
				) : (
					<VerifyResultsTable results={results} onRowUpdated={onRowUpdated} />
				)}
			</CardContent>
		</Card>
	);
}

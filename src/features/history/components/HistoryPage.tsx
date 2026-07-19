import { Link } from "@tanstack/react-router";
import { save } from "@tauri-apps/plugin-dialog";
import { Inbox } from "lucide-react";
import { toast } from "sonner";
import { exportResultsToCsv } from "@/features/batch-verify/api/verifyBatchEmails";
import { Button } from "@/shared/components/ui/button";
import { Input } from "@/shared/components/ui/input";
import {
	Select,
	SelectContent,
	SelectGroup,
	SelectItem,
	SelectTrigger,
	SelectValue,
} from "@/shared/components/ui/select";
import { Skeleton } from "@/shared/components/ui/skeleton";
import {
	Table,
	TableBody,
	TableCell,
	TableHead,
	TableHeader,
	TableRow,
} from "@/shared/components/ui/table";
import { VerifyResultsTable } from "@/shared/components/verify-results-table";
import type { VerifyResult } from "@/shared/types/verify-result";
import { countHistory, fetchHistory } from "../api/fetchHistory";
import { HISTORY_PAGE_SIZE, useHistory } from "../hooks/useHistory";

function HistoryTableSkeleton() {
	return (
		<div className="overflow-x-auto rounded-lg border">
			<Table>
				<TableHeader>
					<TableRow>
						<TableHead>邮箱</TableHead>
						<TableHead>验证时间</TableHead>
						<TableHead>状态</TableHead>
						<TableHead>SMTP 响应码</TableHead>
						<TableHead>Catch-all</TableHead>
						<TableHead>操作</TableHead>
					</TableRow>
				</TableHeader>
				<TableBody>
					{Array.from({ length: 8 }, (_, i) => (
						// biome-ignore lint/suspicious/noArrayIndexKey: static skeleton placeholder rows, never reordered/added/removed individually
						<TableRow key={i}>
							<TableCell>
								<Skeleton className="h-4 w-40" />
							</TableCell>
							<TableCell>
								<Skeleton className="h-4 w-32" />
							</TableCell>
							<TableCell>
								<Skeleton className="h-5 w-14 rounded-full" />
							</TableCell>
							<TableCell>
								<Skeleton className="h-4 w-8" />
							</TableCell>
							<TableCell>
								<Skeleton className="h-4 w-6" />
							</TableCell>
							<TableCell>
								<Skeleton className="h-6 w-14" />
							</TableCell>
						</TableRow>
					))}
				</TableBody>
			</Table>
		</div>
	);
}

export function HistoryPage() {
	const {
		results,
		total,
		domains,
		loading,
		domainFilter,
		setDomainFilter,
		emailSearch,
		setEmailSearch,
		page,
		setPage,
		resetFilters,
		reload,
	} = useHistory();

	const totalPages = Math.max(1, Math.ceil(total / HISTORY_PAGE_SIZE));

	const handleRowUpdated = (_updated: VerifyResult) => {
		void reload();
	};

	const handleExport = async (all: boolean) => {
		const filterParams = all
			? {}
			: { domainFilter, emailSearch: emailSearch || undefined };
		const count = await countHistory(filterParams);
		if (count === 0) {
			toast.error("没有可导出的记录");
			return;
		}
		const rows = await fetchHistory({ ...filterParams, limit: count });

		const path = await save({
			defaultPath: "history-export.csv",
			filters: [{ name: "CSV", extensions: ["csv"] }],
		});
		if (!path) return;
		try {
			await exportResultsToCsv(rows, path);
			toast.success(`已导出到 ${path}`);
		} catch (e) {
			toast.error(e instanceof Error ? e.message : "导出失败");
		}
	};

	const domainItems = [
		{ label: "全部域名", value: "all" },
		...domains.map((d) => ({ label: d, value: d })),
	];

	return (
		<div className="flex flex-col gap-6">
			<div className="flex flex-wrap items-center gap-2">
				<Select
					items={domainItems}
					value={domainFilter ?? "all"}
					onValueChange={(v) =>
						setDomainFilter(!v || v === "all" ? undefined : v)
					}
				>
					<SelectTrigger className="w-48">
						<SelectValue />
					</SelectTrigger>
					<SelectContent>
						<SelectGroup>
							{domainItems.map((item) => (
								<SelectItem key={item.value} value={item.value}>
									{item.label}
								</SelectItem>
							))}
						</SelectGroup>
					</SelectContent>
				</Select>
				<Input
					className="w-64"
					placeholder="搜索邮箱地址"
					value={emailSearch}
					onChange={(e) => setEmailSearch(e.target.value)}
				/>
				<Button variant="ghost" onClick={resetFilters}>
					重置筛选
				</Button>
			</div>

			{loading ? (
				<HistoryTableSkeleton />
			) : results.length === 0 ? (
				<div className="flex flex-col items-center justify-center gap-3 rounded-lg border border-dashed py-16 text-muted-foreground">
					<Inbox className="size-8" />
					<span>还没有验证记录</span>
					<div className="flex gap-2">
						<Button
							render={<Link to="/single" />}
							nativeButton={false}
							variant="outline"
							size="sm"
						>
							去验证一个邮箱
						</Button>
						<Button
							render={<Link to="/batch" />}
							nativeButton={false}
							variant="outline"
							size="sm"
						>
							去批量验证
						</Button>
					</div>
				</div>
			) : (
				<div className="relative">
					<VerifyResultsTable
						results={results}
						onRowUpdated={handleRowUpdated}
					/>
					<div className="mt-3 flex items-center justify-between">
						<span className="text-sm text-muted-foreground">
							共 {total} 条，第 {page + 1} / {totalPages} 页
						</span>
						<div className="flex gap-2">
							<Button
								variant="outline"
								size="sm"
								disabled={page === 0}
								onClick={() => setPage(page - 1)}
							>
								上一页
							</Button>
							<Button
								variant="outline"
								size="sm"
								disabled={page + 1 >= totalPages}
								onClick={() => setPage(page + 1)}
							>
								下一页
							</Button>
						</div>
					</div>
					<div className="mt-3 flex gap-2">
						<Button variant="outline" onClick={() => handleExport(false)}>
							导出CSV（当前筛选）
						</Button>
						<Button variant="outline" onClick={() => handleExport(true)}>
							导出CSV（全部结果）
						</Button>
					</div>
				</div>
			)}
		</div>
	);
}

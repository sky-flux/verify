import { useState } from "react";
import { Button } from "@/shared/components/ui/button";
import {
	ToggleGroup,
	ToggleGroupItem,
} from "@/shared/components/ui/toggle-group";
import { VerifyResultsTable } from "@/shared/components/verify-results-table";
import type { Verdict, VerifyResult } from "@/shared/types/verify-result";

const FILTERS: { value: Verdict | "all"; label: string }[] = [
	{ value: "all", label: "全部" },
	{ value: "Valid", label: "有效" },
	{ value: "Invalid", label: "无效" },
	{ value: "Unknown", label: "未知" },
	{ value: "RiskyCatchAll", label: "Catch-all" },
];

const PAGE_SIZE = 20;

export function BatchResultsTable({
	data,
	filter,
	onFilterChange,
	onRowUpdated,
}: {
	data: VerifyResult[];
	filter: Verdict | "all";
	onFilterChange: (filter: Verdict | "all") => void;
	onRowUpdated?: (updated: VerifyResult) => void;
}) {
	const [pageIndex, setPageIndex] = useState(0);

	const filteredData =
		filter === "all" ? data : data.filter((r) => r.verdict === filter);
	const pageCount = Math.max(1, Math.ceil(filteredData.length / PAGE_SIZE));
	const currentPage = Math.min(pageIndex, pageCount - 1);
	const pageData = filteredData.slice(
		currentPage * PAGE_SIZE,
		(currentPage + 1) * PAGE_SIZE,
	);

	const handleFilterChange = (v: Verdict | "all") => {
		onFilterChange(v);
		setPageIndex(0);
	};

	return (
		<div className="flex flex-col gap-3">
			<ToggleGroup
				value={[filter]}
				onValueChange={(v) =>
					v[0] && handleFilterChange(v[0] as Verdict | "all")
				}
			>
				{FILTERS.map((f) => (
					<ToggleGroupItem key={f.value} value={f.value}>
						{f.label}
					</ToggleGroupItem>
				))}
			</ToggleGroup>

			<VerifyResultsTable results={pageData} onRowUpdated={onRowUpdated} />

			<div className="flex items-center justify-between text-sm">
				<span>
					共 {filteredData.length} 条，第 {currentPage + 1} / {pageCount} 页
				</span>
				<div className="flex gap-2">
					<Button
						variant="outline"
						size="sm"
						disabled={currentPage === 0}
						onClick={() => setPageIndex(currentPage - 1)}
					>
						上一页
					</Button>
					<Button
						variant="outline"
						size="sm"
						disabled={currentPage + 1 >= pageCount}
						onClick={() => setPageIndex(currentPage + 1)}
					>
						下一页
					</Button>
				</div>
			</div>
		</div>
	);
}

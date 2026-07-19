import { Eye, RefreshCw } from "lucide-react";
import { useState } from "react";
import { VerdictBadge } from "@/features/batch-verify/components/VerdictBadge";
import { verifySingleEmail } from "@/features/single-verify/api/verifySingleEmail";
import { Button } from "@/shared/components/ui/button";
import type { VerifyResult } from "@/shared/types/verify-result";
import { type ResizableColumnDef, ResizableTable } from "./resizable-table";
import { VerifyResultDetailSheet } from "./verify-result-detail-sheet";

/**
 * The single table used to present verification results everywhere they
 * appear — History (paginated), Batch Verify (paginated, filterable), and
 * the Dashboard's recent-activity card (unpaginated, top N) — so all three
 * share identical columns, resize behavior, and view/reverify actions.
 * Pagination and filtering are page-level concerns and stay outside this
 * component; callers just pass whatever slice of results should be shown.
 */
export function VerifyResultsTable({
	results,
	onRowUpdated,
}: {
	results: VerifyResult[];
	onRowUpdated?: (updated: VerifyResult) => void;
}) {
	const [selected, setSelected] = useState<VerifyResult | null>(null);
	const [sheetOpen, setSheetOpen] = useState(false);
	const [reverifyingId, setReverifyingId] = useState<string | null>(null);

	const handleReverify = async (email: string, id: string) => {
		setReverifyingId(id);
		try {
			const updated = await verifySingleEmail(email, id);
			onRowUpdated?.(updated);
			setSelected(updated);
		} finally {
			setReverifyingId(null);
		}
	};

	const openDetail = (r: VerifyResult) => {
		setSelected(r);
		setSheetOpen(true);
	};

	const columns: ResizableColumnDef<VerifyResult>[] = [
		{
			key: "email",
			label: "邮箱",
			defaultWidthPercent: 32,
			render: (r) => r.email,
		},
		{
			key: "checkedAt",
			label: "验证时间",
			defaultWidthPercent: 21,
			render: (r) => new Date(r.checkedAt).toLocaleString(),
		},
		{
			key: "verdict",
			label: "状态",
			defaultWidthPercent: 11,
			render: (r) =>
				reverifyingId === r.id ? (
					<RefreshCw className="size-4 animate-spin" />
				) : (
					<VerdictBadge verdict={r.verdict} />
				),
		},
		{
			key: "smtpCode",
			label: "SMTP 响应码",
			defaultWidthPercent: 14,
			render: (r) => r.smtpCode ?? "—",
		},
		{
			key: "catchAll",
			label: "Catch-all",
			defaultWidthPercent: 11,
			render: (r) => (r.catchAll === null ? "-" : r.catchAll ? "是" : "否"),
		},
		{
			key: "actions",
			label: "操作",
			defaultWidthPercent: 11,
			cellClassName: "flex gap-1 overflow-hidden",
			stopClickPropagation: true,
			render: (r) => (
				<>
					<Button
						variant="ghost"
						size="icon-sm"
						title="查看详情"
						onClick={() => openDetail(r)}
					>
						<Eye />
					</Button>
					<Button
						variant="ghost"
						size="icon-sm"
						title="重新验证"
						disabled={reverifyingId === r.id}
						onClick={() => handleReverify(r.email, r.id)}
					>
						<RefreshCw />
					</Button>
				</>
			),
		},
	];

	return (
		<>
			<ResizableTable
				columns={columns}
				rows={results}
				getRowKey={(r) => r.id}
				onRowClick={openDetail}
			/>

			<VerifyResultDetailSheet
				result={selected}
				open={sheetOpen}
				onOpenChange={setSheetOpen}
				onReverify={handleReverify}
				reverifying={reverifyingId === selected?.id}
			/>
		</>
	);
}

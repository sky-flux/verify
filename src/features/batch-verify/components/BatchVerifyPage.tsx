import { save } from "@tauri-apps/plugin-dialog";
import {
	isPermissionGranted,
	requestPermission,
	sendNotification,
} from "@tauri-apps/plugin-notification";
import { useEffect, useState } from "react";
import { toast } from "sonner";
import {
	AlertDialog,
	AlertDialogAction,
	AlertDialogCancel,
	AlertDialogContent,
	AlertDialogDescription,
	AlertDialogFooter,
	AlertDialogHeader,
	AlertDialogTitle,
	AlertDialogTrigger,
} from "@/shared/components/ui/alert-dialog";
import { Button } from "@/shared/components/ui/button";
import type { Verdict } from "@/shared/types/verify-result";
import { exportResultsToCsv } from "../api/verifyBatchEmails";
import { useBatchStore } from "../store/batchStore";
import { BatchInputPanel } from "./BatchInputPanel";
import { BatchProgressBar } from "./BatchProgressBar";
import { BatchResultsTable } from "./BatchResultsTable";
import { BatchSummaryCards } from "./BatchSummaryCards";

async function notifyDone() {
	if (document.visibilityState === "visible") {
		toast.success("批量验证已完成");
		return;
	}
	let granted = await isPermissionGranted();
	if (!granted) granted = (await requestPermission()) === "granted";
	if (granted)
		sendNotification({ title: "SKY FLUX VERIFY", body: "批量验证已完成" });
}

export function BatchVerifyPage() {
	const {
		batchStatus,
		batchProgress,
		batchResults,
		batchSummary,
		batchError,
		startBatch,
		cancelBatch,
		resetBatch,
		updateResult,
	} = useBatchStore();
	const [restartOpen, setRestartOpen] = useState(false);
	const [filter, setFilter] = useState<Verdict | "all">("all");

	useEffect(() => {
		if (batchError) toast.error(batchError);
	}, [batchError]);

	const handleStart = async (emails: string[]) => {
		await startBatch(emails);
		// startBatch never throws (it catches and sets batchError), so check
		// the resulting status rather than a try/catch here — a failed batch
		// must not also claim success via this notification.
		if (useBatchStore.getState().batchStatus === "done") await notifyDone();
	};

	const handleExport = async (all: boolean) => {
		const path = await save({
			defaultPath: "verify-results.csv",
			filters: [{ name: "CSV", extensions: ["csv"] }],
		});
		if (!path) return;
		const rows = all
			? batchResults
			: batchResults.filter((r) => filter === "all" || r.verdict === filter);
		try {
			await exportResultsToCsv(rows, path);
			toast.success(`已导出到 ${path}`);
		} catch (e) {
			toast.error(e instanceof Error ? e.message : "导出失败");
		}
	};

	return (
		<div className="flex flex-col gap-6">
			{batchStatus === "idle" && <BatchInputPanel onStart={handleStart} />}

			{(batchStatus === "running" || batchStatus === "cancelling") && (
				<>
					<BatchProgressBar
						completed={batchProgress.completed}
						total={batchProgress.total}
						cancelling={batchStatus === "cancelling"}
						onCancel={cancelBatch}
					/>
					<BatchResultsTable
						data={batchResults}
						filter={filter}
						onFilterChange={setFilter}
						onRowUpdated={updateResult}
					/>
				</>
			)}

			{batchStatus === "done" && batchSummary && (
				<>
					<BatchSummaryCards summary={batchSummary} />
					<BatchResultsTable
						data={batchResults}
						filter={filter}
						onFilterChange={setFilter}
						onRowUpdated={updateResult}
					/>
					<div className="flex gap-2">
						<Button variant="outline" onClick={() => handleExport(false)}>
							导出CSV（当前筛选）
						</Button>
						<Button variant="outline" onClick={() => handleExport(true)}>
							导出CSV（全部结果）
						</Button>
						<AlertDialog open={restartOpen} onOpenChange={setRestartOpen}>
							<AlertDialogTrigger render={<Button variant="secondary" />}>
								重新开始一批新的
							</AlertDialogTrigger>
							<AlertDialogContent>
								<AlertDialogHeader>
									<AlertDialogTitle>确定要开始新一批验证吗？</AlertDialogTitle>
									<AlertDialogDescription>
										结果已保存到历史记录，可以在"历史记录"页面找回。
									</AlertDialogDescription>
								</AlertDialogHeader>
								<AlertDialogFooter>
									<AlertDialogCancel>取消</AlertDialogCancel>
									<AlertDialogAction onClick={resetBatch}>
										确定
									</AlertDialogAction>
								</AlertDialogFooter>
							</AlertDialogContent>
						</AlertDialog>
					</div>
				</>
			)}
		</div>
	);
}

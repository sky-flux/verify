import { Check, CheckCircle2, MinusCircle, XCircle } from "lucide-react";
import { VerdictBadge } from "@/features/batch-verify/components/VerdictBadge";
import { Button } from "@/shared/components/ui/button";
import {
	Card,
	CardContent,
	CardHeader,
	CardTitle,
} from "@/shared/components/ui/card";
import { Separator } from "@/shared/components/ui/separator";
import type { VerifyResult } from "@/shared/types/verify-result";

function CheckRow({
	status,
	label,
}: {
	status: "ok" | "fail" | "neutral";
	label: string;
}) {
	return (
		<div className="flex items-center gap-2 text-sm">
			{status === "ok" && <CheckCircle2 className="size-4 text-green-600" />}
			{status === "fail" && <XCircle className="size-4 text-red-600" />}
			{status === "neutral" && (
				<MinusCircle className="size-4 text-muted-foreground" />
			)}
			<span>{label}</span>
		</div>
	);
}

export function ResultCard({
	result,
	copied,
	onCopy,
	onReverify,
	onClear,
}: {
	result: VerifyResult;
	copied: boolean;
	onCopy: () => void;
	onReverify: () => void;
	onClear: () => void;
}) {
	return (
		<Card className="animate-in fade-in slide-in-from-bottom-2 duration-200">
			<CardHeader className="flex flex-row items-center justify-between gap-2">
				<CardTitle className="text-lg font-semibold break-all">
					{result.email}
				</CardTitle>
				<VerdictBadge verdict={result.verdict} />
			</CardHeader>
			<CardContent className="flex flex-col gap-4">
				<div className="flex flex-col gap-2">
					<CheckRow
						status={result.syntaxValid ? "ok" : "fail"}
						label="语法校验"
					/>
					<CheckRow
						status={result.mxFound ? "ok" : "fail"}
						label={`MX 记录${result.mxRecords.length ? `: ${result.mxRecords.join(", ")}` : ""}`}
					/>
					<CheckRow
						status={
							result.catchAll === null
								? "neutral"
								: result.catchAll
									? "fail"
									: "ok"
						}
						label={
							result.catchAll === null
								? "Catch-all: 不适用"
								: result.catchAll
									? "Catch-all: 是（结果不可信）"
									: "Catch-all: 否"
						}
					/>
				</div>
				<Separator />
				<div className="flex flex-col gap-1 text-sm text-muted-foreground">
					<span>SMTP 响应码: {result.smtpCode ?? "无"}</span>
					<span className="font-mono text-xs break-all">
						{result.smtpMessage || result.error || "—"}
					</span>
					<span>验证耗时: {result.durationMs}ms</span>
				</div>
				<div className="flex gap-2">
					<Button variant="outline" size="sm" onClick={onCopy}>
						{copied ? (
							<>
								<Check data-icon="inline-start" />
								已复制
							</>
						) : (
							"复制结果"
						)}
					</Button>
					<Button variant="outline" size="sm" onClick={onReverify}>
						重新验证
					</Button>
					<Button variant="ghost" size="sm" onClick={onClear}>
						清空
					</Button>
				</div>
			</CardContent>
		</Card>
	);
}

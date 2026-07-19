import { RefreshCw } from "lucide-react";
import { VerdictBadge } from "@/features/batch-verify/components/VerdictBadge";
import { Button } from "@/shared/components/ui/button";
import {
	Sheet,
	SheetContent,
	SheetDescription,
	SheetHeader,
	SheetTitle,
} from "@/shared/components/ui/sheet";
import type { VerifyResult } from "@/shared/types/verify-result";

export function VerifyResultDetailSheet({
	result,
	open,
	onOpenChange,
	onReverify,
	reverifying,
}: {
	result: VerifyResult | null;
	open: boolean;
	onOpenChange: (open: boolean) => void;
	onReverify: (email: string, id: string) => void;
	reverifying: boolean;
}) {
	if (!result) return null;
	return (
		<Sheet open={open} onOpenChange={onOpenChange}>
			<SheetContent className="flex flex-col gap-4 overflow-y-auto p-6">
				<SheetHeader className="p-0">
					<SheetTitle className="break-all">{result.email}</SheetTitle>
					<SheetDescription>验证记录详情</SheetDescription>
				</SheetHeader>

				<VerdictBadge verdict={result.verdict} className="w-fit" />

				<dl className="flex flex-col gap-2 text-sm">
					<div>
						<dt className="text-muted-foreground">UUID</dt>
						<dd className="font-mono text-xs break-all">{result.id}</dd>
					</div>
					<div>
						<dt className="text-muted-foreground">MX 记录</dt>
						<dd>
							{result.mxRecords.length ? result.mxRecords.join(", ") : "无"}
						</dd>
					</div>
					<div>
						<dt className="text-muted-foreground">SMTP 响应码</dt>
						<dd>{result.smtpCode ?? "无"}</dd>
					</div>
					<div>
						<dt className="text-muted-foreground">原始服务器消息</dt>
						<dd className="font-mono text-xs break-all">
							{result.smtpMessage || "—"}
						</dd>
					</div>
					{result.error && (
						<div>
							<dt className="text-muted-foreground">错误</dt>
							<dd className="text-destructive">{result.error}</dd>
						</div>
					)}
					<div>
						<dt className="text-muted-foreground">验证时间</dt>
						<dd>{new Date(result.checkedAt).toLocaleString()}</dd>
					</div>
				</dl>

				<Button
					variant="outline"
					disabled={reverifying}
					onClick={() => onReverify(result.email, result.id)}
				>
					<RefreshCw
						data-icon="inline-start"
						className={reverifying ? "animate-spin" : ""}
					/>
					重新验证
				</Button>
			</SheetContent>
		</Sheet>
	);
}

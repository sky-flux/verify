import { writeText } from "@tauri-apps/plugin-clipboard-manager";
import { MailSearch } from "lucide-react";
import { useState } from "react";
import {
	Alert,
	AlertDescription,
	AlertTitle,
} from "@/shared/components/ui/alert";
import { Skeleton } from "@/shared/components/ui/skeleton";
import { useSingleVerify } from "../hooks/useSingleVerify";
import { useHandoffStore } from "../store/handoffStore";
import { ResultCard } from "./ResultCard";
import { SingleVerifyForm } from "./SingleVerifyForm";

export function SingleVerifyPage() {
	// Only read once, at mount — consumeResult() clears the store, and
	// useSingleVerify only looks at its initial value on first render too.
	const [handoffResult] = useState(() =>
		useHandoffStore.getState().consumeResult(),
	);
	const { status, result, error, verify, reverify, reset } =
		useSingleVerify(handoffResult);
	const [copied, setCopied] = useState(false);

	const handleCopy = async () => {
		if (!result) return;
		const text = `${result.email} — ${result.verdict} (SMTP ${result.smtpCode ?? "N/A"})\n${result.smtpMessage}`;
		await writeText(text);
		setCopied(true);
		setTimeout(() => setCopied(false), 2000);
	};

	return (
		<div className="flex flex-col gap-6">
			<SingleVerifyForm loading={status === "loading"} onSubmit={verify} />

			{status === "idle" && (
				<div className="flex flex-col items-center justify-center gap-2 rounded-lg border border-dashed py-16 text-muted-foreground">
					<MailSearch className="size-8" />
					<span>输入邮箱地址开始验证</span>
				</div>
			)}

			{status === "loading" && <Skeleton className="h-64 w-full rounded-lg" />}

			{status === "error" && (
				<Alert variant="destructive">
					<AlertTitle>验证失败</AlertTitle>
					<AlertDescription>{error}</AlertDescription>
				</Alert>
			)}

			{status === "result" && result && (
				<ResultCard
					result={result}
					copied={copied}
					onCopy={handleCopy}
					onReverify={reverify}
					onClear={reset}
				/>
			)}
		</div>
	);
}

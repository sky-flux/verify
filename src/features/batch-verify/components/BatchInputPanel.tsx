import { open } from "@tauri-apps/plugin-dialog";
import { readTextFile } from "@tauri-apps/plugin-fs";
import { TriangleAlert } from "lucide-react";
import { useState } from "react";
import { toast } from "sonner";
import {
	Alert,
	AlertDescription,
	AlertTitle,
} from "@/shared/components/ui/alert";
import { Button } from "@/shared/components/ui/button";
import { Card } from "@/shared/components/ui/card";
import {
	Tabs,
	TabsContent,
	TabsList,
	TabsTrigger,
} from "@/shared/components/ui/tabs";
import { Textarea } from "@/shared/components/ui/textarea";
import { parseImportedEmails } from "../api/verifyBatchEmails";

export function BatchInputPanel({
	onStart,
}: {
	onStart: (emails: string[]) => void;
}) {
	const [text, setText] = useState("");
	const [dragActive, setDragActive] = useState(false);

	// Pure input-echo, not validation: counting/deduping raw lines to show the
	// user "N addresses detected" is UI feedback, not a decision about
	// validity — the authoritative syntax check still happens in Rust per
	// verify_single_email/verify_batch_emails.
	// No useMemo — the React Compiler memoizes this automatically.
	const emails = (() => {
		const seen = new Set<string>();
		const result: string[] = [];
		for (const line of text.split("\n")) {
			const trimmed = line.trim();
			if (!trimmed) continue;
			const key = trimmed.toLowerCase();
			if (!seen.has(key)) {
				seen.add(key);
				result.push(trimmed);
			}
		}
		return result;
	})();

	const importParsedEmails = async (rawContent: string) => {
		try {
			const parsed = await parseImportedEmails(rawContent);
			setText(parsed.join("\n"));
		} catch (e) {
			toast.error(e instanceof Error ? e.message : "解析文件失败");
		}
	};

	const handleFile = async (path: string) => {
		try {
			const content = await readTextFile(path);
			await importParsedEmails(content);
		} catch {
			toast.error("无法读取文件");
		}
	};

	return (
		<div className="flex flex-col gap-4">
			<Alert>
				<TriangleAlert />
				<AlertTitle>批量探测合规提醒</AlertTitle>
				<AlertDescription>
					<p>
						批量高频对同一批目标邮件服务器做 SMTP
						探测，有被目标服务器判定为滥用行为、进而拉黑你所在网络出口 IP
						的风险。
					</p>
					<p>
						请自行确认使用场景是否符合当地及目标邮箱服务商的相关法规（如
						GDPR、CAN-SPAM
						等），批量验证他人邮箱涉及个人数据处理，建议仅用于验证你已通过合法渠道（如官网公开信息、名片、对方主动提供）获取的邮箱，不用于未经同意的大规模数据收集场景。
					</p>
				</AlertDescription>
			</Alert>

			<Tabs defaultValue="paste">
				<TabsList>
					<TabsTrigger value="paste">粘贴文本</TabsTrigger>
					<TabsTrigger value="file">导入文件</TabsTrigger>
				</TabsList>
				<TabsContent value="paste" className="flex flex-col gap-2">
					<Textarea
						className="min-h-48 font-mono text-sm"
						placeholder="每行一个邮箱地址"
						value={text}
						onChange={(e) => setText(e.target.value)}
					/>
					<span className="text-sm text-muted-foreground">
						检测到 {emails.length} 个邮箱地址（已自动去重）
					</span>
				</TabsContent>
				<TabsContent value="file">
					<Card
						className={`flex h-48 flex-col items-center justify-center gap-2 border-dashed text-center transition-colors ${dragActive ? "border-solid bg-muted" : ""}`}
						onDragOver={(e) => {
							e.preventDefault();
							setDragActive(true);
						}}
						onDragLeave={() => setDragActive(false)}
						onDrop={async (e) => {
							e.preventDefault();
							setDragActive(false);
							const file = e.dataTransfer.files[0];
							if (file) {
								const content = await file.text();
								await importParsedEmails(content);
							}
						}}
					>
						<span className="text-muted-foreground">
							拖拽 CSV/TXT 文件到这里，或
						</span>
						<Button
							type="button"
							variant="outline"
							onClick={async () => {
								const path = await open({
									multiple: false,
									filters: [{ name: "邮箱列表", extensions: ["csv", "txt"] }],
								});
								if (typeof path === "string") await handleFile(path);
							}}
						>
							选择文件
						</Button>
					</Card>
				</TabsContent>
			</Tabs>
			<Button
				size="lg"
				disabled={emails.length === 0}
				onClick={() => onStart(emails)}
			>
				开始验证
			</Button>
		</div>
	);
}

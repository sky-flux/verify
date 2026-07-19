import { Button } from "@/shared/components/ui/button";
import { Progress } from "@/shared/components/ui/progress";

export function BatchProgressBar({
	completed,
	total,
	cancelling,
	onCancel,
}: {
	completed: number;
	total: number;
	cancelling: boolean;
	onCancel: () => void;
}) {
	const pct = total > 0 ? (completed / total) * 100 : 0;
	return (
		<div className="flex items-center gap-4">
			<div className="flex-1">
				<Progress value={pct} />
				<span className="mt-1 block text-sm text-muted-foreground">
					已完成 {completed} / {total}
				</span>
			</div>
			<Button variant="destructive" disabled={cancelling} onClick={onCancel}>
				{cancelling ? "正在停止..." : "停止"}
			</Button>
		</div>
	);
}

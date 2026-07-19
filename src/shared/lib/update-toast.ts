import type { Update } from "@tauri-apps/plugin-updater";
import { toast } from "sonner";
import { useUpdateStore } from "@/shared/store/updateStore";

/**
 * Single place that renders the "update available" notification so the
 * silent startup check and the manual Settings button can't drift from each
 * other (they previously built two slightly different toasts for the same
 * event).
 */
export function notifyUpdateAvailable(update: Update) {
	toast.info(`发现新版本 ${update.version}`, {
		description: update.body ?? undefined,
		duration: Number.POSITIVE_INFINITY,
		action: {
			label: "立即更新",
			onClick: () => {
				void useUpdateStore
					.getState()
					.installUpdate()
					.then((installed) => {
						if (!installed) toast.error("更新失败，请稍后重试");
					})
					.catch(() => {
						toast.error("更新失败，请稍后重试");
					});
			},
		},
	});
}

import { relaunch } from "@tauri-apps/plugin-process";
import { check } from "@tauri-apps/plugin-updater";
import { useCallback, useState } from "react";

export function useAppUpdate() {
	const [checking, setChecking] = useState(false);
	const [installing, setInstalling] = useState(false);
	const [version, setVersion] = useState<string | null>(null);
	const [notes, setNotes] = useState<string | null>(null);

	const checkForUpdate = useCallback(async () => {
		setChecking(true);
		try {
			const update = await check();
			if (update) {
				setVersion(update.version);
				setNotes(update.body ?? null);
			}
			return update;
		} finally {
			setChecking(false);
		}
	}, []);

	const installUpdate = useCallback(async () => {
		setInstalling(true);
		try {
			const update = await check();
			if (!update) return false;
			await update.downloadAndInstall();
			await relaunch();
			return true;
		} finally {
			setInstalling(false);
		}
	}, []);

	return {
		checking,
		installing,
		version,
		notes,
		checkForUpdate,
		installUpdate,
	};
}

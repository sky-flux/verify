import { relaunch } from "@tauri-apps/plugin-process";
import { check, type Update } from "@tauri-apps/plugin-updater";
import { create } from "zustand";

interface UpdateState {
	checking: boolean;
	installing: boolean;
	update: Update | null;
	checkForUpdate: () => Promise<Update | null>;
	installUpdate: () => Promise<boolean>;
}

/**
 * Shared across every consumer (the silent startup check in RootLayout and
 * the manual button in Settings) so both read the same in-flight/found state
 * instead of running independent checks. installUpdate() reuses the cached
 * `update` from the last checkForUpdate() call rather than re-checking, so a
 * click always installs exactly the version that was shown to the user.
 */
export const useUpdateStore = create<UpdateState>((set, get) => ({
	checking: false,
	installing: false,
	update: null,

	checkForUpdate: async () => {
		set({ checking: true });
		try {
			const update = await check();
			set({ update });
			return update;
		} finally {
			set({ checking: false });
		}
	},

	installUpdate: async () => {
		set({ installing: true });
		try {
			const update = get().update ?? (await get().checkForUpdate());
			if (!update) return false;
			await update.downloadAndInstall();
			await relaunch();
			return true;
		} finally {
			set({ installing: false });
		}
	},
}));

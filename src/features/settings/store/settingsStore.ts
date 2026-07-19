import { z } from "zod";
import { create } from "zustand";
import { AppCommandError, callCommand } from "@/shared/lib/tauri";
import { type Settings, settingsSchema } from "@/shared/types/verify-result";

interface SettingsState {
	settings: Settings | null;
	loading: boolean;
	fieldErrors: Record<string, string>;
	load: () => Promise<void>;
	save: (settings: Settings) => Promise<boolean>;
}

/**
 * Mirrors persisted state in memory. Persistence itself lives entirely in
 * Rust (`tauri-plugin-store`, wrapped by the `get_settings`/`update_settings`
 * commands) — this store is a cache the UI reads/writes through, never a
 * second source of truth, per the "frontend never talks to plugin storage
 * directly" rule from the project's core principle.
 */
export const useSettingsStore = create<SettingsState>((set) => ({
	settings: null,
	loading: false,
	fieldErrors: {},

	load: async () => {
		set({ loading: true });
		const settings = await callCommand("get_settings", settingsSchema);
		set({ settings, loading: false });
	},

	save: async (settings) => {
		set({ loading: true, fieldErrors: {} });
		try {
			await callCommand("update_settings", z.null(), { settings });
			set({ settings, loading: false });
			return true;
		} catch (error) {
			const message = error instanceof Error ? error.message : String(error);
			const field = error instanceof AppCommandError ? error.field : null;
			set({ loading: false, fieldErrors: { [field ?? "_global"]: message } });
			return false;
		}
	},
}));

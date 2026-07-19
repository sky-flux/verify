import { useEffect } from "react";
import { useSettingsStore } from "../store/settingsStore";

/** Public read hook other features use to reach settings state without
 * importing `settingsStore` internals directly (keeps feature boundaries
 * clean per this project's feature-based module rules). */
export function useSettings() {
	const settings = useSettingsStore((s) => s.settings);
	const loading = useSettingsStore((s) => s.loading);
	const load = useSettingsStore((s) => s.load);

	useEffect(() => {
		if (!settings && !loading) void load();
	}, [settings, loading, load]);

	return { settings, loading };
}

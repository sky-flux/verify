import { create } from "zustand";
import type { NetworkHealth } from "@/shared/types/verify-result";
import { checkNetworkHealth } from "../api/getDashboardStats";

interface NetworkHealthState {
	health: NetworkHealth | null;
	checking: boolean;
	recheck: () => Promise<void>;
}

/**
 * A single shared result so the Sidebar footer indicator and the Dashboard
 * header badge never disagree — both read this store instead of each
 * running their own `check_network_health` call.
 */
export const useNetworkHealthStore = create<NetworkHealthState>((set) => ({
	health: null,
	checking: false,
	recheck: async () => {
		set({ checking: true });
		try {
			const health = await checkNetworkHealth();
			set({ health, checking: false });
		} catch {
			set({ checking: false });
		}
	},
}));

export function useNetworkHealth() {
	const { health, checking, recheck } = useNetworkHealthStore();
	return { health, checking, recheck };
}

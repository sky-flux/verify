import { useEffect, useState } from "react";
import type { VerifyResult } from "@/shared/types/verify-result";
import {
	countHistory,
	fetchDistinctDomains,
	fetchHistory,
} from "../api/fetchHistory";

export const HISTORY_PAGE_SIZE = 20;
const PAGE_SIZE = HISTORY_PAGE_SIZE;
const SEARCH_DEBOUNCE_MS = 300;

export function useHistory() {
	const [domainFilter, setDomainFilterState] = useState<string | undefined>(
		undefined,
	);
	// The input's displayed value updates every keystroke; the debounced
	// value below is what actually drives the query, so typing doesn't fire
	// a Rust LIKE query (and a loading-state flash) on every character.
	const [emailSearchInput, setEmailSearchInput] = useState("");
	const [emailSearch, setEmailSearch] = useState("");
	const [page, setPage] = useState(0);
	const [results, setResults] = useState<VerifyResult[]>([]);
	const [total, setTotal] = useState(0);
	const [domains, setDomains] = useState<string[]>([]);
	const [loading, setLoading] = useState(false);

	useEffect(() => {
		const timer = setTimeout(() => {
			setEmailSearch(emailSearchInput);
			setPage(0);
		}, SEARCH_DEBOUNCE_MS);
		return () => clearTimeout(timer);
	}, [emailSearchInput]);

	const reload = async () => {
		setLoading(true);
		try {
			const filterParams = {
				domainFilter,
				emailSearch: emailSearch || undefined,
			};
			const [data, count] = await Promise.all([
				fetchHistory({
					...filterParams,
					limit: PAGE_SIZE,
					offset: page * PAGE_SIZE,
				}),
				countHistory(filterParams),
			]);
			setResults(data);
			setTotal(count);
		} finally {
			setLoading(false);
		}
	};

	// Depends on the actual primitive filter/page values, not `reload`'s
	// function identity — `reload` is redefined every render (its identity
	// isn't guaranteed stable; relying on that caused an infinite reload
	// loop in `tauri dev`'s live transform, where the React Compiler pass
	// doesn't run the same way it does for a production `vite build`), but
	// the effect only needs to fire when what it fetches actually changes.
	// biome-ignore lint/correctness/useExhaustiveDependencies: intentionally depends on the primitive filter/page values, not on `reload`'s (unstable) function identity — see comment above.
	useEffect(() => {
		void reload();
	}, [domainFilter, emailSearch, page]);

	useEffect(() => {
		void fetchDistinctDomains().then(setDomains);
	}, []);

	const resetFilters = () => {
		setDomainFilterState(undefined);
		setEmailSearchInput("");
		setEmailSearch("");
		setPage(0);
	};

	return {
		results,
		total,
		domains,
		loading,
		domainFilter,
		setDomainFilter: (v: string | undefined) => {
			setDomainFilterState(v);
			setPage(0);
		},
		emailSearch: emailSearchInput,
		setEmailSearch: setEmailSearchInput,
		page,
		setPage,
		resetFilters,
		reload,
	};
}

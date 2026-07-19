import {
	createRootRoute,
	createRoute,
	createRouter,
} from "@tanstack/react-router";
import { BatchVerifyPage } from "@/features/batch-verify";
import { DashboardPage } from "@/features/dashboard";
import { HistoryPage } from "@/features/history";
import { SettingsPage } from "@/features/settings";
import { SingleVerifyPage } from "@/features/single-verify";
import { RootLayout } from "./RootLayout";

const rootRoute = createRootRoute({ component: RootLayout });

const indexRoute = createRoute({
	getParentRoute: () => rootRoute,
	path: "/",
	component: DashboardPage,
});

const singleRoute = createRoute({
	getParentRoute: () => rootRoute,
	path: "/single",
	component: SingleVerifyPage,
});

const batchRoute = createRoute({
	getParentRoute: () => rootRoute,
	path: "/batch",
	component: BatchVerifyPage,
});

const historyRoute = createRoute({
	getParentRoute: () => rootRoute,
	path: "/history",
	component: HistoryPage,
});

const settingsRoute = createRoute({
	getParentRoute: () => rootRoute,
	path: "/settings",
	component: SettingsPage,
});

const routeTree = rootRoute.addChildren([
	indexRoute,
	singleRoute,
	batchRoute,
	historyRoute,
	settingsRoute,
]);

export const router = createRouter({ routeTree });

declare module "@tanstack/react-router" {
	interface Register {
		router: typeof router;
	}
}

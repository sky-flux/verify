import { Outlet, useRouterState } from "@tanstack/react-router";
import { Circle } from "lucide-react";
import { useEffect } from "react";
import { useNetworkHealth } from "@/features/dashboard";
import {
	SidebarInset,
	SidebarProvider,
	SidebarTrigger,
} from "@/shared/components/ui/sidebar";
import { Toaster } from "@/shared/components/ui/sonner";
import { TooltipProvider } from "@/shared/components/ui/tooltip";
import { notifyUpdateAvailable } from "@/shared/lib/update-toast";
import { useUpdateStore } from "@/shared/store/updateStore";
import { AppSidebar } from "./AppSidebar";
import { ROUTE_TITLES } from "./routeTitles";

function ContentHeader() {
	const pathname = useRouterState({ select: (s) => s.location.pathname });
	const { health, checking, recheck } = useNetworkHealth();

	useEffect(() => {
		if (!health && !checking) void recheck();
	}, [health, checking, recheck]);

	return (
		<header className="flex h-12 shrink-0 items-center gap-3 border-b px-4">
			<SidebarTrigger />
			<h1 className="text-lg font-semibold">{ROUTE_TITLES[pathname] ?? ""}</h1>
			<span
				className={`ml-auto flex items-center gap-1.5 text-sm ${health?.port25Reachable ? "text-green-600" : "text-red-600"}`}
			>
				<Circle className="size-2 fill-current" />
				{checking
					? "检测中..."
					: health?.port25Reachable
						? "网络就绪"
						: "25端口不可用"}
			</span>
		</header>
	);
}

function UpdateChecker() {
	const checkForUpdate = useUpdateStore((s) => s.checkForUpdate);

	useEffect(() => {
		// Silent background check — swallow failures instead of surfacing
		// them, so an offline launch doesn't produce a startup error toast.
		void checkForUpdate()
			.then((update) => {
				if (update) notifyUpdateAvailable(update);
			})
			.catch(() => {});
	}, [checkForUpdate]);

	return null;
}

export function RootLayout() {
	return (
		<TooltipProvider>
			<UpdateChecker />
			{/* shadcn's default wrapper is `min-h-svh` (grows with content),
			    which leaves no bounded scroll region — the whole window would
			    scroll instead of just `main`. Locking it to `h-screen` (100vh —
			    `svh` isn't reliably supported in every WebKit build this app
			    ships in) and `min-h-0` down the flex chain makes `main`'s
			    `overflow-y-auto` the only thing that scrolls, keeping the
			    header fixed. */}
			<SidebarProvider className="h-screen">
				<AppSidebar />
				{/* min-w-0 is needed alongside min-h-0: SidebarInset/main are flex
				    items in SidebarProvider's row, and flex items default to
				    min-width: auto, so a wide descendant (e.g. a resizable table
				    wider than the viewport) pushes this item — and the whole row,
				    dragging the sidebar with it — wider instead of being contained
				    by main's own overflow-x-auto. */}
				<SidebarInset className="min-h-0 min-w-0">
					<ContentHeader />
					<main className="min-h-0 min-w-0 flex-1 overflow-y-auto p-6">
						<Outlet />
					</main>
				</SidebarInset>
			</SidebarProvider>
			<Toaster />
		</TooltipProvider>
	);
}

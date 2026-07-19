import { Link, useRouterState } from "@tanstack/react-router";
import {
	History,
	LayoutDashboard,
	ListChecks,
	Mail,
	Settings,
} from "lucide-react";
import {
	Sidebar,
	SidebarContent,
	SidebarGroup,
	SidebarGroupContent,
	SidebarHeader,
	SidebarMenu,
	SidebarMenuButton,
	SidebarMenuItem,
	SidebarSeparator,
} from "@/shared/components/ui/sidebar";
import { ROUTE_TITLES } from "./routeTitles";

const NAV_ITEMS = [
	{ to: "/", icon: LayoutDashboard },
	{ to: "/single", icon: Mail },
	{ to: "/batch", icon: ListChecks },
	{ to: "/history", icon: History },
] as const;

export function AppSidebar() {
	const pathname = useRouterState({ select: (s) => s.location.pathname });

	return (
		<Sidebar collapsible="icon" className="overflow-x-hidden">
			<SidebarHeader>
				<div className="flex items-center overflow-hidden px-2 py-1">
					<span className="min-w-0 overflow-hidden font-semibold whitespace-nowrap opacity-100 transition-[width,opacity] duration-300 ease-[cubic-bezier(0.4,0,0.2,1)] group-data-[collapsible=icon]:w-0 group-data-[collapsible=icon]:opacity-0">
						SKY FLUX VERIFY
					</span>
				</div>
			</SidebarHeader>

			<SidebarContent>
				<SidebarGroup>
					<SidebarGroupContent>
						<SidebarMenu>
							{NAV_ITEMS.map((item) => (
								<SidebarMenuItem key={item.to}>
									<SidebarMenuButton
										render={<Link to={item.to} />}
										isActive={pathname === item.to}
										tooltip={ROUTE_TITLES[item.to]}
									>
										<item.icon />
										<span>{ROUTE_TITLES[item.to]}</span>
									</SidebarMenuButton>
								</SidebarMenuItem>
							))}
						</SidebarMenu>
					</SidebarGroupContent>
				</SidebarGroup>

				<SidebarSeparator />

				<SidebarGroup>
					<SidebarGroupContent>
						<SidebarMenu>
							<SidebarMenuItem>
								<SidebarMenuButton
									render={<Link to="/settings" />}
									isActive={pathname === "/settings"}
									tooltip={ROUTE_TITLES["/settings"]}
								>
									<Settings />
									<span>{ROUTE_TITLES["/settings"]}</span>
								</SidebarMenuButton>
							</SidebarMenuItem>
						</SidebarMenu>
					</SidebarGroupContent>
				</SidebarGroup>
			</SidebarContent>
		</Sidebar>
	);
}

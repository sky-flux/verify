/** Single source of truth for each route's page title, shared by the
 * sidebar nav labels (AppSidebar) and the fixed content-area header
 * (RootLayout) so they can never drift out of sync. */
export const ROUTE_TITLES: Record<string, string> = {
	"/": "仪表盘",
	"/single": "邮件验证",
	"/batch": "批量验证",
	"/history": "历史记录",
	"/settings": "设置",
};

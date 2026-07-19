import { useForm } from "@tanstack/react-form";
import { getVersion } from "@tauri-apps/api/app";
import { appDataDir } from "@tauri-apps/api/path";
import { openPath, openUrl } from "@tauri-apps/plugin-opener";
import { useEffect, useState } from "react";
import { toast } from "sonner";
import { Button } from "@/shared/components/ui/button";
import {
	Card,
	CardContent,
	CardHeader,
	CardTitle,
} from "@/shared/components/ui/card";
import {
	Field,
	FieldDescription,
	FieldError,
	FieldGroup,
	FieldLabel,
} from "@/shared/components/ui/field";
import { Input } from "@/shared/components/ui/input";
import {
	Select,
	SelectContent,
	SelectGroup,
	SelectItem,
	SelectTrigger,
	SelectValue,
} from "@/shared/components/ui/select";
import { Slider } from "@/shared/components/ui/slider";
import { useAppUpdate } from "@/shared/hooks/use-app-update";
import type { DnsResolver } from "@/shared/types/verify-result";
import { useSettingsStore } from "../store/settingsStore";

const DNS_RESOLVER_ITEMS: { label: string; value: DnsResolver }[] = [
	{ label: "系统 DNS", value: "system" },
	{ label: "Cloudflare (1.1.1.1)", value: "cloudflare" },
	{ label: "Google (8.8.8.8)", value: "google" },
];

const AUTHOR_EMAIL = "martinadams.dev@gmail.com";
const REPO_URL = "https://github.com/sky-flux/verify";

export function SettingsForm() {
	const { settings, load, save, fieldErrors } = useSettingsStore();
	const [saving, setSaving] = useState(false);
	const [appVersion, setAppVersion] = useState("");
	const { checking, installing, checkForUpdate, installUpdate } =
		useAppUpdate();

	useEffect(() => {
		if (!settings) void load();
	}, [settings, load]);

	useEffect(() => {
		void getVersion().then(setAppVersion);
	}, []);

	const form = useForm({
		defaultValues: settings ?? {
			heloDomain: "sky-flux-verify.local",
			smtpTimeoutSeconds: 10,
			domainCooldownSeconds: 3,
			maxConcurrentDomains: 5,
			dnsResolver: "cloudflare" as DnsResolver,
		},
		onSubmit: async ({ value }) => {
			setSaving(true);
			const ok = await save(value);
			setSaving(false);
			if (ok) toast.success("设置已保存");
		},
	});

	if (!settings) return null;

	return (
		<form
			className="flex flex-col gap-6"
			onSubmit={(e) => {
				e.preventDefault();
				void form.handleSubmit();
			}}
		>
			<Card>
				<CardHeader>
					<CardTitle>探测参数</CardTitle>
				</CardHeader>
				<CardContent>
					<FieldGroup>
						<form.Field name="heloDomain">
							{(field) => (
								<Field data-invalid={!!fieldErrors.heloDomain}>
									<FieldLabel htmlFor="heloDomain">HELO 域名</FieldLabel>
									<Input
										id="heloDomain"
										aria-invalid={!!fieldErrors.heloDomain}
										value={field.state.value}
										onChange={(e) => field.handleChange(e.target.value)}
									/>
									{fieldErrors.heloDomain && (
										<FieldError>{fieldErrors.heloDomain}</FieldError>
									)}
								</Field>
							)}
						</form.Field>

						<form.Field name="dnsResolver">
							{(field) => (
								<Field>
									<FieldLabel htmlFor="dnsResolver">DNS 解析方式</FieldLabel>
									<Select
										items={DNS_RESOLVER_ITEMS}
										value={field.state.value}
										onValueChange={(v) =>
											v && field.handleChange(v as DnsResolver)
										}
									>
										<SelectTrigger id="dnsResolver" className="w-full">
											<SelectValue />
										</SelectTrigger>
										<SelectContent>
											<SelectGroup>
												{DNS_RESOLVER_ITEMS.map((item) => (
													<SelectItem key={item.value} value={item.value}>
														{item.label}
													</SelectItem>
												))}
											</SelectGroup>
										</SelectContent>
									</Select>
									<FieldDescription>
										部分家庭宽带/路由器的 DNS
										会对不存在的域名返回错误结果（劫持），影响验证准确性。推荐使用
										Cloudflare 或 Google 公共
										DNS；如需连接公司内网邮件服务器，请选择系统 DNS。
									</FieldDescription>
								</Field>
							)}
						</form.Field>

						<form.Field name="smtpTimeoutSeconds">
							{(field) => (
								<Field data-invalid={!!fieldErrors.smtpTimeoutSeconds}>
									<FieldLabel htmlFor="smtpTimeoutSeconds">
										SMTP 超时时间（秒）
									</FieldLabel>
									<Input
										id="smtpTimeoutSeconds"
										type="number"
										aria-invalid={!!fieldErrors.smtpTimeoutSeconds}
										value={field.state.value}
										onChange={(e) => field.handleChange(Number(e.target.value))}
									/>
									{fieldErrors.smtpTimeoutSeconds && (
										<FieldError>{fieldErrors.smtpTimeoutSeconds}</FieldError>
									)}
								</Field>
							)}
						</form.Field>

						<form.Field name="domainCooldownSeconds">
							{(field) => (
								<Field data-invalid={!!fieldErrors.domainCooldownSeconds}>
									<FieldLabel>
										域名探测冷却间隔: {field.state.value}s
									</FieldLabel>
									<Slider
										min={1}
										max={10}
										step={1}
										aria-invalid={!!fieldErrors.domainCooldownSeconds}
										value={[field.state.value]}
										onValueChange={(v) =>
											field.handleChange(Array.isArray(v) ? v[0] : v)
										}
									/>
									{fieldErrors.domainCooldownSeconds && (
										<FieldError>{fieldErrors.domainCooldownSeconds}</FieldError>
									)}
								</Field>
							)}
						</form.Field>

						<form.Field name="maxConcurrentDomains">
							{(field) => (
								<Field data-invalid={!!fieldErrors.maxConcurrentDomains}>
									<FieldLabel>最大并发域名数: {field.state.value}</FieldLabel>
									<Slider
										min={1}
										max={20}
										step={1}
										aria-invalid={!!fieldErrors.maxConcurrentDomains}
										value={[field.state.value]}
										onValueChange={(v) =>
											field.handleChange(Array.isArray(v) ? v[0] : v)
										}
									/>
									<FieldDescription>
										设置过高容易被目标邮件服务器判定滥用
									</FieldDescription>
									{fieldErrors.maxConcurrentDomains && (
										<FieldError>{fieldErrors.maxConcurrentDomains}</FieldError>
									)}
								</Field>
							)}
						</form.Field>

						{fieldErrors._global && (
							<FieldError>{fieldErrors._global}</FieldError>
						)}
					</FieldGroup>
				</CardContent>
			</Card>

			<Card>
				<CardHeader>
					<CardTitle>应用信息</CardTitle>
				</CardHeader>
				<CardContent className="flex flex-col gap-3">
					<span className="text-sm text-muted-foreground">
						版本 {appVersion}
					</span>
					<div className="flex flex-col gap-1 text-sm text-muted-foreground">
						<button
							type="button"
							className="w-fit text-left hover:text-foreground hover:underline"
							onClick={() => openUrl(`mailto:${AUTHOR_EMAIL}`)}
						>
							作者邮箱：{AUTHOR_EMAIL}
						</button>
						<button
							type="button"
							className="w-fit text-left hover:text-foreground hover:underline"
							onClick={() => openUrl(REPO_URL)}
						>
							项目地址：{REPO_URL}
						</button>
					</div>
					<div className="flex gap-2">
						<Button
							type="button"
							variant="outline"
							className="w-fit"
							onClick={async () => {
								try {
									await openPath(await appDataDir());
								} catch {
									toast.error("无法打开数据目录");
								}
							}}
						>
							打开数据目录
						</Button>
						<Button
							type="button"
							variant="outline"
							className="w-fit"
							disabled={checking || installing}
							onClick={async () => {
								try {
									const update = await checkForUpdate();
									if (!update) {
										toast.success("已是最新版本");
										return;
									}
									toast.info(`发现新版本 ${update.version}`, {
										action: {
											label: "立即更新",
											onClick: () => {
												void installUpdate().catch(() => {
													toast.error("更新失败，请稍后重试");
												});
											},
										},
									});
								} catch {
									toast.error("检查更新失败");
								}
							}}
						>
							{checking ? "检查中..." : installing ? "更新中..." : "检查更新"}
						</Button>
					</div>
				</CardContent>
			</Card>

			<div className="sticky bottom-0 flex justify-end bg-background/80 py-3 backdrop-blur">
				<Button type="submit" disabled={saving}>
					{saving ? "保存中..." : "保存"}
				</Button>
			</div>
		</form>
	);
}

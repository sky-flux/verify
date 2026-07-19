# SKY FLUX VERIFY

[English](./README.md)

一个跨平台桌面应用（Windows / macOS / Linux），用于验证邮箱地址是否真实存在，核心技术手段是 SMTP 握手探测（`RCPT TO` 层面的真实校验），不依赖第三方付费 API。因为是本地桌面应用直接跑在用户自己的网络出口，不像 Cloudflare Workers 那样被官方封锁 25 端口，可以做完整的 SMTP 探测。

基于 **Tauri v2** 构建，遵循「能用 Rust 解决的逻辑，全部放在 Rust 里」的原则——邮箱语法校验、MX 查询、SMTP 探测、catch-all 判定、限流冷却、批量任务调度、CSV 导入导出等业务逻辑全部在 Rust 后端实现，前端只负责 UI 渲染。

## 功能

- **单个邮箱验证** —— 输入一个邮箱，立即返回语法/MX/SMTP 响应码/catch-all 校验结果。
- **批量邮箱验证** —— 粘贴或导入 CSV/TXT 邮箱列表，实时查看验证进度，完成后导出结果为 CSV。
- **历史记录** —— 每次验证结果本地持久化（SQLite），支持按域名/邮箱筛选、重新验证、导出。
- **仪表盘** —— 汇总统计指标与最近验证记录。
- **设置** —— HELO 域名、超时时间、并发数等参数配置，经 `tauri-plugin-store` 持久化。

## 技术栈

- **应用框架**：[Tauri v2](https://tauri.app/)（Rust 后端 + WebView 前端渲染）
- **后端**：Rust、`tokio`、`hickory-resolver`（MX 查询）、`sqlx`（SQLite）
- **前端**：Vite + React 19 + TypeScript、TanStack Router/Form、Zustand、Tailwind CSS
- **UI 组件库**：[shadcn/ui](https://ui.shadcn.com/)，底层 primitive 使用 [Base UI](https://base-ui.com/)

## 下载

每次打 tag 发布时，会在 [Releases](https://github.com/sky-flux/verify/releases) 页面发布 macOS（Apple 芯片 & Intel 芯片）、Windows、Linux 的预编译安装包。

## 开发

前置依赖：[Bun](https://bun.sh/)、[Rust](https://www.rust-lang.org/tools/install)，以及对应系统的 [Tauri v2 环境依赖](https://v2.tauri.app/start/prerequisites/)。

```bash
bun install
bun run tauri dev
```

## 打包构建

```bash
bun run tauri build
```

产物在 `src-tauri/target/release/bundle/` 目录下。

## 项目结构

```
src/                  # 前端（按功能切片：single-verify / batch-verify / history / dashboard / settings）
src-tauri/src/
  domain/             # 验证核心逻辑：语法校验、MX、SMTP、catch-all、限流、批量调度
  infra/              # SQLite 持久化、CSV 导出
  commands/           # 暴露给前端的 Tauri command
```

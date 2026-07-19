# SKY FLUX VERIFY

[中文文档](./README.zh-CN.md)

A cross-platform desktop app (Windows / macOS / Linux) for verifying whether an email address really exists, using live SMTP handshake probing (`RCPT TO`) instead of a paid third-party API. Because it's a native desktop app running on your own network egress, it isn't blocked on port 25 the way cloud functions (e.g. Cloudflare Workers) typically are, so it can perform a full SMTP probe.

Built with **Tauri v2** — all verification logic (syntax validation, MX lookup, SMTP probing, catch-all detection, rate limiting, batch scheduling, CSV import/export) lives in the Rust backend. The frontend is purely a rendering layer.

## Features

- **Single email verification** — enter one address, get an instant result (syntax / MX / SMTP response code / catch-all).
- **Batch verification** — paste a list or import a CSV/TXT file, watch live progress, then export results to CSV.
- **History** — every verification is persisted locally (SQLite); browse, filter by domain/email, re-verify, and export.
- **Dashboard** — summary stats and the most recent verifications at a glance.
- **Settings** — configurable HELO domain, timeout, and concurrency, persisted via `tauri-plugin-store`.

## Tech Stack

- **App shell**: [Tauri v2](https://tauri.app/) (Rust backend + WebView frontend)
- **Backend**: Rust, `tokio`, `hickory-resolver` (MX lookups), `sqlx` (SQLite)
- **Frontend**: Vite + React 19 + TypeScript, TanStack Router/Form, Zustand, Tailwind CSS
- **UI**: [shadcn/ui](https://ui.shadcn.com/) on top of [Base UI](https://base-ui.com/) primitives

## Download

Prebuilt binaries for macOS (Apple Silicon & Intel), Windows, and Linux are published on the [Releases](https://github.com/sky-flux/verify/releases) page for every tagged version.

## Development

Prerequisites: [Bun](https://bun.sh/), [Rust](https://www.rust-lang.org/tools/install), and the [Tauri v2 system prerequisites](https://v2.tauri.app/start/prerequisites/) for your OS.

```bash
bun install
bun run tauri dev
```

## Building

```bash
bun run tauri build
```

Bundles are produced under `src-tauri/target/release/bundle/`.

## Project Structure

```
src/                  # frontend (feature-sliced: single-verify, batch-verify, history, dashboard, settings)
src-tauri/src/
  domain/             # verification logic: syntax, MX, SMTP, catch-all, rate limiting, batch scheduling
  infra/              # SQLite persistence, CSV export
  commands/           # Tauri commands exposed to the frontend
```

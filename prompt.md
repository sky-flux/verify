# SKY FLUX VERIFY —— Tauri 桌面版 SMTP 邮箱验证工具 —— 完整开发 Prompt

> 这份文档可以直接作为 Prompt 交给 Claude Code / Cursor / 其他 AI 编码工具，或者作为你自己开发时的技术规格说明书使用。项目名：**SKY FLUX VERIFY**。（本文档已合并技术架构与UI/UX设计两部分，单文件维护。）

## 一、项目目标

**SKY FLUX VERIFY** 是一个跨平台桌面应用（Windows / macOS / Linux），用于验证邮箱地址是否真实存在，核心技术手段是 SMTP 握手探测（RCPT TO 层面的真实校验，不依赖第三方付费 API）。因为是本地桌面应用直接跑在用户自己的网络出口，不像 Cloudflare Workers 那样被官方封锁 25 端口，可以做完整的 SMTP 探测。

必须支持两种模式：
1. **单个邮箱验证**——输入一个邮箱，立即返回验证结果
2. **批量邮箱验证**——粘贴/导入一批邮箱（文本框粘贴、或导入 CSV/TXT 文件），逐个验证并展示进度，验证完成后可导出结果为 CSV

### 核心原则：前后端职责边界

> **能用 Rust 解决的逻辑，全部放在 Rust 里；前端只负责 UI 渲染和用户交互，不承担任何业务逻辑。**

具体到这个项目：
- 邮箱语法校验、MX查询、SMTP探测、catch-all判定、限流冷却、批量任务调度、CSV解析/导出、灰名单重试、汇总统计计算——这些**全部**是 Rust 后端的职责，前端不重复实现任何一份逻辑（哪怕是"轻量的正则校验"这种看起来无害的东西也不例外，避免前后端出现两份不一致的校验规则）。
- 前端的职责严格限定在：渲染表单/表格/进度条这些 UI 元素、捕获用户操作（点击、粘贴、拖拽文件）、调用 Tauri command 把操作转发给 Rust、把 Rust 返回的结果渲染出来。TanStack Table 的排序/筛选/分页这类"对已经拿到手的数据做纯展示层面的重新排列"不算业务逻辑，可以留在前端；但凡是涉及"判断/计算/决策"的都要下放到 Rust。
- 好处：逻辑只有一份实现，不会出现"前端校验通过了但后端报错"这种体验割裂，也方便以后如果要复用这套核心逻辑做别的客户端（比如CLI工具）时直接调用同一份 Rust 代码。

## 二、技术栈

- **框架**：Tauri v2（Rust 后端 + Web 前端渲染，二进制体积小、性能好，避免 Electron 的臃肿）
- **后端语言**：Rust
- **前端语言/构建**：Vite + React 19 + TypeScript
- **UI 组件库**：shadcn/ui，**底层 primitive 选用 Base UI**（而不是默认的 Radix Primitives）——Base UI 是 MUI 团队和原 Radix 作者合流后推出的无样式组件库，shadcn/ui 目前已支持在生成组件时指定 Base UI 作为底层，组件源码同样是直接拷进项目，方便按需定制；Tailwind CSS 负责样式
- **全局状态管理**：Zustand——管理批量验证的实时进度、当前结果集、设置项（HELO域名/超时/并发数等跨页面共享的状态）
- **路由**：TanStack Router（类型安全的路由，文件式或代码式均可，用于单邮箱页/批量页/历史记录页/设置页之间的导航）
- **表单**：TanStack Form——用于单邮箱输入校验、设置页表单（超时时间、并发数等数值型输入的校验与联动）
- **表格**：TanStack Table——批量验证结果表格的核心，需要支持排序（按状态/耗时排序）、筛选（按 verdict 筛选 valid/invalid/unknown/catch-all）、分页（批量数量大时避免一次性渲染卡顿）、列显隐控制
- **代码规范/格式化**：Biome（替代 ESLint + Prettier 的一体化工具链，速度更快，配置更简单）
- **异步运行时（Rust侧）**：`tokio`
- **DNS 解析**：`hickory-resolver`（原 `trust-dns-resolver`，已改名，功能不变，用于查询 MX 记录；这个没有对应的 Tauri 官方插件，仍需直接用这个 crate）
- **打包分发**：Tauri 自带的 `tauri build`，产出 `.msi`/`.dmg`/`.AppImage`

### 优先使用 Tauri 官方插件

能用官方插件解决的功能，一律不要自己手搓或引入第三方 crate/npm 包，减少维护成本和安全面：

| 功能 | 官方插件 | 用途 |
|---|---|---|
| 本地数据持久化（验证结果、catch-all判定缓存、历史记录） | `tauri-plugin-sql`（内置 SQLite driver） | 替代直接依赖 `rusqlite`。**注意**：插件本身提供前端可直接发SQL的JS API，但本项目遵循"前端仅渲染UI和交互"的原则，所有SQL查询一律封装成 Rust 侧的 Tauri command，前端不直接调用插件的SQL能力，保证数据访问逻辑（查询条件拼接、TTL过期判断等）只有一份实现 |
| 应用设置持久化 | `tauri-plugin-store` | 存 HELO域名/超时/并发数这些设置项，替代手写"读写JSON配置文件"或者只存在 Zustand 的 `persist` middleware 里；Store 是 Tauri 官方维护的 key-value 持久化方案，前后端都能访问 |
| 文件选择/保存对话框（导入邮箱列表、导出CSV） | `tauri-plugin-dialog` | 已在前面提到 |
| 文件读写（CSV解析、导出） | `tauri-plugin-fs` | 已在前面提到，配合 `tauri-plugin-dialog` 拿到路径后读写 |
| 批量验证完成时系统通知 | `tauri-plugin-notification` | 批量任务耗时可能较长（几百上千个邮箱），验证完成后弹系统通知提醒用户，不用一直盯着窗口 |
| 日志记录 | `tauri-plugin-log` | 记录SMTP探测过程中的异常/超时/连接失败，方便排查问题，比自己写日志系统省事 |
| 一键复制验证结果 | `tauri-plugin-clipboard-manager`（可选） | 单邮箱验证结果页加个"复制"按钮，方便粘贴到开发信/CRM里 |
| 打开文件所在目录/用默认程序打开文件 | `tauri-plugin-opener` | Tauri v2 里替代旧版 `shell.open` 的官方插件，设置页"打开数据目录"按钮用它在系统文件管理器里定位到SQLite文件 |
| 应用自动更新（如果要长期维护迭代） | `tauri-plugin-updater`（可选，加分项） | 后续版本迭代时不用用户手动下载新安装包 |

### 前端项目脚手架命令参考

```bash
bun create vite@latest verify -- --template react-ts
cd verify

# shadcn/ui 初始化（会自动装 Tailwind 并生成 components.json）
# 初始化时选择 Base UI 作为底层 primitive（shadcn CLI 交互式提问里选，
# 或者查 shadcn 官方文档确认当前版本指定 Base UI 的具体参数/registry写法，
# 这个能力比较新，CLI flag 可能随版本迭代变化，以官方文档为准）
bunx shadcn@latest init

# 核心依赖
bun add zustand
bun add @tanstack/react-router @tanstack/router-devtools
bun add @tanstack/react-form
bun add @tanstack/react-table

# Biome（替代 ESLint+Prettier）
bun add --dev --exact @biomejs/biome
bunx @biomejs/biome init

# 接入 Tauri（交互式提问里 App name 填 "SKY FLUX VERIFY"，
# identifier 用 com.sky-flux.verify）
bun add --dev @tauri-apps/cli
bunx tauri init

# Tauri 官方插件（前端JS包 + Rust侧同步注册，两边都要装）
bun add @tauri-apps/api
bun add @tauri-apps/plugin-dialog @tauri-apps/plugin-fs
bun add @tauri-apps/plugin-sql @tauri-apps/plugin-store
bun add @tauri-apps/plugin-notification @tauri-apps/plugin-log
bun add @tauri-apps/plugin-clipboard-manager   # 可选

# Rust 侧对应 crate（在 src-tauri 目录下，cargo本身不受包管理器选择影响，仍用cargo）
cd src-tauri
cargo add tauri-plugin-sql --features sqlite
cargo add tauri-plugin-store
cargo add tauri-plugin-dialog
cargo add tauri-plugin-fs
cargo add tauri-plugin-notification
cargo add tauri-plugin-log
cargo add tauri-plugin-clipboard-manager   # 可选
```

日常开发命令也统一换成 bun：`bun run dev`、`bun run build`、`bunx tauri dev`、`bunx tauri build`。如果项目里已经有 `package-lock.json` 或 `yarn.lock` 残留，记得删掉，避免和 `bun.lockb` 混用导致依赖不一致。

> **关于 identifier 里带连字符**：`com.sky-flux.verify` 没问题，Tauri 的 identifier 校验规则允许字母、数字、连字符（-）和点（.），只要求每一段不能以数字开头。macOS 的 Bundle Identifier 规范同样允许连字符。唯一需要注意的例外是——如果以后想用 Tauri Mobile 出 Android 版本，Android 的 applicationId 是照搬 Java 包名规则，**不允许连字符**，到时候 Android 那一份配置需要单独改成 `com.skyflux.verify`（去掉连字符），桌面端（Windows/macOS/Linux）不受影响，可以放心用 `com.sky-flux.verify`。

装完之后记得在 `src-tauri/src/lib.rs`（或 `main.rs`）的 `tauri::Builder` 链式调用里逐个 `.plugin(tauri_plugin_xxx::init())` 注册，并在 `src-tauri/capabilities/` 的权限配置里给对应窗口开放这些插件的权限（Tauri v2 的权限系统默认最小化授权，插件装了不代表自动能用，还要显式声明 capability）。

## 三、核心 SMTP 探测逻辑（Rust 后端）

严格按以下握手流程实现，全程只做到 `RCPT TO` 这一步，**绝不发送 `DATA`**，即不真正投递邮件内容：

```
建立 TCP 连接到目标邮件服务器的 25 端口
读取: 220 服务器欢迎语
发送: EHLO yourdomain.com
读取: 250 服务器能力列表（可能是多行 250-xxx 250 xxx）
发送: MAIL FROM:<verify@yourdomain.com>
读取: 250 OK
发送: RCPT TO:<目标邮箱>
读取: 250/550/451 等响应码 ← 这是判断依据
发送: QUIT
```

响应码含义：
- `250`/`251` → 邮箱存在（valid）
- `550`/`551`/`553` → 邮箱不存在（invalid）
- `450`/`451`/`452` → 临时拒绝（可能是灰名单/限流，建议标记为 unknown 并支持稍后重试）
- `421` → 服务暂时不可用
- 连接失败/超时 → unknown，很可能是对方按 IP 信誉直接拒绝，或者目标网络有防护

### Rust 后端代码组织（遵循社区最佳实践的分层结构）

不要把所有模块平铺成一堆同级 `.rs` 文件，按"职责分层"组织成目录，`domain/` 层不依赖 `tauri` crate，保证核心逻辑能脱离Tauri单独测试、以后也方便复用（比如抽出去做CLI工具）：

```
src-tauri/
  src/
    main.rs                  # 极薄，只调用 lib.rs 的 run()（Tauri v2 推荐写法）
    lib.rs                   # 应用入口：注册插件、注册command、组装 AppState

    commands/                # Tauri command 层——只做参数转发+调用domain层+错误转换，不写业务逻辑
      mod.rs
      verify.rs              # verify_single_email / verify_batch_emails / cancel_batch_verification
      history.rs              # fetch_history
      settings.rs             # get_settings / update_settings

    domain/                  # 核心业务逻辑，零Tauri依赖，理论上可脱离Tauri单独测试/复用
      mod.rs
      dns.rs                  # MX记录查询：用 hickory-resolver 查MX，按preference排序；无MX记录时按RFC5321回退查A记录
      smtp.rs                 # SMTP握手探测：tokio::net::TcpStream原始连接，手写EHLO→MAIL FROM→RCPT TO→QUIT，
                               # 不用高层邮件发送库（不支持"只握手到RCPT TO就停"这种半截操作），超时8-10秒
      catch_all.rs            # catch-all判定：探测一个随机生成的邮箱（uuid::Uuid::new_v4()拼域名即可，
                               # 这个一次性探测地址不入库不用v7），返回250则判定该域名catch-all
      rate_limiter.rs          # 限流冷却：按域名维护"上次探测时间戳"，同域名两次探测间隔2-3秒，
                               # 用 Arc<Mutex<HashMap<>>> 或 dashmap 处理并发访问
      batch.rs                 # 批量调度：tokio::task并发+限制同时探测的域名数量（比如最多5个），
                               # 通过 ProgressReporter trait（见下方"事件通知与Tauri解耦"）上报进度，
                               # 支持 tokio_util::sync::CancellationToken 取消
      types.rs                 # VerifyResult / Verdict / BatchSummary / Settings 等核心数据结构

    infra/                   # 基础设施层：跟外部世界打交道的具体实现
      mod.rs
      db.rs                   # SQLite repository，封装 tauri-plugin-sql 的调用（verify_results表增删查、
                               # catch_all_cache表读写、UUID v7主键生成）
      csv_export.rs            # CSV解析/导出，配合 tauri-plugin-fs 读写文件

    state.rs                  # AppState：rate_limiter的共享状态、批量任务的取消令牌等，通过.manage()注册进Tauri
    error.rs                   # 统一错误类型 AppError（用 thiserror 定义，需手动实现 serde::Serialize 才能传回前端）
```

**`domain/` 层与 Tauri 解耦的关键写法**——批量调度模块需要"上报进度给前端"，但不应该在 `domain/batch.rs` 里直接依赖 `tauri::AppHandle`，用一个 trait 抽象掉这层依赖：

```rust
// domain/batch.rs —— 纯业务逻辑，不知道Tauri的存在，可以直接单元测试
pub trait ProgressReporter: Send + Sync {
    fn report(&self, completed: u32, total: u32);
}

pub async fn run_batch(
    emails: Vec<String>,
    reporter: &dyn ProgressReporter,
) -> (Vec<VerifyResult>, BatchSummary) {
    // ...批量探测逻辑，每完成一个调用 reporter.report(...)
}
```
```rust
// commands/verify.rs —— Tauri相关的粘合代码只放在这一层
struct TauriProgressReporter(tauri::AppHandle);
impl ProgressReporter for TauriProgressReporter {
    fn report(&self, completed: u32, total: u32) {
        let _ = self.0.emit("verify-progress", (completed, total));
    }
}

#[tauri::command]
async fn verify_batch_emails(
    app_handle: tauri::AppHandle,
    emails: Vec<String>,
) -> Result<(Vec<VerifyResult>, BatchSummary), AppError> {
    let reporter = TauriProgressReporter(app_handle);
    Ok(domain::batch::run_batch(emails, &reporter).await)
}
```

**统一错误类型，不要到处传 `String`**——用 `thiserror` 定义领域错误，Tauri command 的错误类型需要实现 `Serialize` 才能传回前端：

```rust
// error.rs
#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("DNS查询失败: {0}")]
    DnsLookup(String),
    #[error("SMTP连接失败: {0}")]
    SmtpConnection(String),
    #[error("数据库错误: {0}")]
    Database(#[from] sqlx::Error),
}

impl serde::Serialize for AppError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where S: serde::Serializer {
        serializer.serialize_str(self.to_string().as_ref())
    }
}
```
这样前端拿到的错误至少能区分"是DNS问题还是SMTP问题还是数据库问题"，而不是一个不透明的字符串，比文档早前版本里 `Result<VerifyResult, String>` 的写法更规范。

**AppState 管理跨command共享状态**——不要用裸的全局静态变量，通过 Tauri 的 `.manage()` 注册：
```rust
// state.rs
pub struct AppState {
    pub rate_limiter: Arc<Mutex<HashMap<String, Instant>>>,
    pub cancel_token: Arc<Mutex<Option<CancellationToken>>>,
}
```
```rust
// lib.rs
tauri::Builder::default()
    .manage(AppState::new())
    .invoke_handler(tauri::generate_handler![
        commands::verify::verify_single_email,
        commands::verify::verify_batch_emails,
        commands::verify::cancel_batch_verification,
        commands::history::fetch_history,
        commands::settings::get_settings,
        commands::settings::update_settings,
    ])
```

### Tauri Command 层完整签名

`commands/` 是前端唯一能接触到的入口，所有业务逻辑都封装在 command 内部调用 `domain/`、`infra/` 完成，前端拿到的永远是算好的最终结果：

```rust
// commands/verify.rs
#[tauri::command]
async fn verify_single_email(
    email: String,
    state: tauri::State<'_, AppState>,
) -> Result<VerifyResult, AppError>

#[tauri::command]
async fn verify_batch_emails(
    app_handle: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
    emails: Vec<String>,
) -> Result<(Vec<VerifyResult>, BatchSummary), AppError>
// 返回值把逐条结果和汇总统计一起打包返回，前端不用自己再算一遍统计

#[tauri::command]
async fn cancel_batch_verification(state: tauri::State<'_, AppState>) -> Result<(), AppError>

// commands/history.rs
#[tauri::command]
async fn fetch_history(domain_filter: Option<String>) -> Result<Vec<VerifyResult>, AppError>
// 历史记录页调用这个，而不是前端直接用 tauri-plugin-sql 发 SELECT，域名筛选的查询条件拼接也在Rust里做

#[tauri::command]
async fn export_results_to_csv(results: Vec<VerifyResult>, file_path: String) -> Result<(), AppError>

// commands/settings.rs
#[tauri::command]
async fn get_settings() -> Result<Settings, AppError>

#[tauri::command]
async fn update_settings(settings: Settings) -> Result<(), AppError>
// 设置的读写也走command，内部调用 tauri-plugin-store，数值范围校验也在这里做，前端不直接碰plugin-store的API

// commands/dashboard.rs
#[tauri::command]
async fn get_dashboard_stats() -> Result<DashboardStats, AppError>
// Dashboard页用，内部对 verify_results 表做 COUNT/AVG 等聚合SQL查询，返回算好的统计数字，
// 前端不对历史数组做reduce/filter.length这类统计计算

#[tauri::command]
async fn check_network_health() -> Result<NetworkHealth, AppError>
// 应用启动时的25端口连通性自检，Sidebar footer的网络状态指示器和Dashboard顶部Badge共用这一个结果
```

### Cargo workspace（可选，规模变大时再考虑）

现阶段单 crate + 上面的模块分层已经够用，不需要一开始就拆 workspace。但如果以后这套 SMTP 探测逻辑要被复用（比如做一个独立的命令行版本），可以把 `domain/` 整个目录拆成独立 crate：

```
Cargo.toml                  # workspace root
crates/
  verify-core/              # 纯业务逻辑，零Tauri依赖，对应现在的 domain/
src-tauri/                  # 薄薄的Tauri粘合层，依赖 verify-core
```
因为 `domain/` 从一开始就没有依赖 Tauri，这个拆分到时候几乎是"整个目录搬过去"，不需要重写。

### 测试与代码规范

- **集成测试放 `tests/` 目录，针对 `domain/` 层写**：因为 `domain/` 不依赖 Tauri，可以直接用普通的 `#[tokio::test]` 测 DNS查询、catch-all判定这些逻辑，不需要起完整的Tauri测试环境。
- **Clippy + rustfmt 是标配**：日常开发用 `cargo clippy --all-targets --all-features -- -D warnings` 把警告当错误处理，`cargo fmt` 走默认配置即可。

### 数据结构

```rust
#[derive(serde::Serialize, serde::Deserialize, Clone)]
struct VerifyResult {
    id: String,              // UUID v7 文本表示，SQLite 主键，见下面"SQLite 数据表结构"
    email: String,
    syntax_valid: bool,
    mx_found: bool,
    mx_records: Vec<String>,
    catch_all: Option<bool>,
    smtp_code: Option<u16>,
    smtp_message: String,
    error: Option<String>,
    verdict: Verdict,
    checked_at: String, // ISO8601时间戳
}

#[derive(serde::Serialize, serde::Deserialize, Clone)]
enum Verdict {
    Valid,
    Invalid,
    RiskyCatchAll,
    Unknown,
}

#[derive(serde::Serialize, serde::Deserialize, Clone)]
struct BatchSummary {
    total: u32,
    valid: u32,
    invalid: u32,
    unknown: u32,
    risky_catch_all: u32,
}

#[derive(serde::Serialize, serde::Deserialize, Clone)]
struct DashboardStats {
    total_verified_all_time: u64,
    overall_valid_rate: f32,      // 0.0 ~ 1.0，前端只负责格式化成百分比展示
    catch_all_domain_count: u64,
    verified_today: u64,
}

#[derive(serde::Serialize, serde::Deserialize, Clone)]
struct NetworkHealth {
    port25_reachable: bool,
    checked_at: String, // ISO8601
    detail: Option<String>, // 连接失败时的具体原因，供Sidebar的Popover展示
}
```
`BatchSummary` / `DashboardStats` 都是 Rust 统计好直接返回给前端的聚合结果，前端只负责渲染这几个数字，**不要**让前端对结果数组做 `.filter().length` 这种统计计算——哪怕逻辑很简单，也算是业务逻辑，按"核心原则"应该在 Rust 里做好。

### SQLite 数据表结构：主键统一用 UUID v7

所有需要持久化到 SQLite 的表（验证结果历史记录、catch-all判定缓存），主键一律用 **UUID v7**（不用自增整数，也不用 UUID v4）。选 v7 的原因：v7 的前 48 位是毫秒级时间戳，天然按时间单调递增，写入 SQLite 的 B-tree 索引时不会像 v4 那样因为完全随机导致索引页分裂、写入性能下降，同时又保留了 UUID 全局唯一、不暴露自增行数的优点，兼顾了"分布式友好"和"本地索引友好"两头。

```sql
CREATE TABLE IF NOT EXISTS verify_results (
    id            TEXT PRIMARY KEY,   -- UUID v7，Rust 侧用 uuid::Uuid::now_v7().to_string() 生成后插入
    email         TEXT NOT NULL,
    syntax_valid  INTEGER NOT NULL,   -- SQLite无原生布尔类型，用0/1
    mx_found      INTEGER NOT NULL,
    catch_all     INTEGER,            -- 允许NULL，对应Option<bool>
    smtp_code     INTEGER,
    smtp_message  TEXT,
    error         TEXT,
    verdict       TEXT NOT NULL,      -- 存Verdict枚举的字符串形式
    checked_at    TEXT NOT NULL       -- ISO8601
);
CREATE INDEX IF NOT EXISTS idx_verify_results_email ON verify_results(email);
CREATE INDEX IF NOT EXISTS idx_verify_results_checked_at ON verify_results(checked_at);

CREATE TABLE IF NOT EXISTS catch_all_cache (
    id          TEXT PRIMARY KEY,   -- UUID v7
    domain      TEXT NOT NULL UNIQUE,
    is_catch_all INTEGER NOT NULL,
    checked_at  TEXT NOT NULL
);
```

Rust 侧依赖：
```bash
cargo add uuid --features v7
```
插入新记录时统一用 `uuid::Uuid::now_v7().to_string()` 生成主键，存成 `TEXT` 类型（36字符标准UUID格式），不用 `BLOB(16)` 这种更省空间但调试时不可读的存法——桌面单机应用的数据量级用不到那种极致的存储优化，可读性优先。

## 四、前端功能需求

### shadcn/ui 组件使用规范（必须遵守）

- **`shared/components/ui/` 目录下由 shadcn CLI 生成的组件文件不允许手动修改**（`button.tsx`、`card.tsx`、`input.tsx`等）。这些是脚手架生成的基础组件源码，改动它们会导致以后跑 `bunx shadcn@latest add xxx` 更新/新增组件时出现难以合并的冲突，也会让"这个组件到底是不是shadcn原版"变得模糊。
- **样式覆盖一律通过 `cn()` 工具函数**（`shared/lib/utils.ts` 里 shadcn 脚手架自带生成的 `clsx` + `tailwind-merge` 封装），在使用组件的地方通过 `className` prop 传入要覆盖的 Tailwind 类名，而不是去改 `ui/` 目录里的源码：
  ```tsx
  <Button className={cn("w-full", isDestructiveAction && "bg-red-600 hover:bg-red-700")}>
    停止
  </Button>
  ```
- **单个 shadcn 组件功能不够用时，用多个 shadcn 组件组合封装成新的业务组件，而不是修改 `ui/` 里的源码，也不是从零手写一个新组件**。新组件放在对应 feature 的 `components/` 目录下（不是 `shared/components/ui/`），内部只应该是若干个 shadcn 组件的组合拼装 + 业务逻辑绑定。比如批量验证页的"结果状态Badge"，不是去改 `ui/badge.tsx` 加一个新的 variant，而是在 `features/batch-verify/components/VerdictBadge.tsx` 里用现成的 `Badge` 组件 + `cn()` 传自定义颜色类名组合出来：
  ```tsx
  // features/batch-verify/components/VerdictBadge.tsx
  import { Badge } from "@/shared/components/ui/badge";
  import { cn } from "@/shared/lib/utils";

  export function VerdictBadge({ verdict }: { verdict: Verdict }) {
    const styles: Record<Verdict, string> = {
      Valid: "bg-green-100 text-green-700 hover:bg-green-100",
      Invalid: "bg-red-100 text-red-700 hover:bg-red-100",
      Unknown: "bg-yellow-100 text-yellow-700 hover:bg-yellow-100",
      RiskyCatchAll: "bg-orange-100 text-orange-700 hover:bg-orange-100",
    };
    const labels: Record<Verdict, string> = {
      Valid: "有效", Invalid: "无效", Unknown: "未知", RiskyCatchAll: "Catch-all",
    };
    return <Badge className={cn(styles[verdict])}>{labels[verdict]}</Badge>;
  }
  ```
- **优先直接用 shadcn 已有组件，不自己从零实现 UI 组件**。设计里提到的所有UI元素（进度条、抽屉、下拉筛选、拖拽区域、骨架屏等）落地时第一步永远是 `bunx shadcn@latest add <component>` 看有没有现成的，没有就看能不能拿现成的几个组合出来（比如"拖拽上传区域"可以用 `Card` 加上原生的 drag-and-drop 事件监听拼出来，不需要额外装第三方拖拽库），实在组合不出来的极少数情况（比如很定制化的图标动画）才考虑手写，且要放在具体 feature 内部而不是污染 `shared/ui/`。

### 0. 前端代码组织方式：Feature-based（按功能垂直切分）

不要按"类型"分层（不要搞一个全局 `components/`、一个全局 `hooks/`、一个全局 `store/` 把所有功能的东西混在一起）。改成按**功能模块（feature）**垂直切分，每个 feature 自己的组件、hooks、API调用封装、类型定义、甚至专属的 Zustand store 都放在自己文件夹里，做到"改一个功能只需要在一个文件夹里动"：

```
src/
  app/                        # 应用级入口，不属于任何具体feature
    router.tsx                # TanStack Router 的路由树注册、根 Provider 组装
    providers.tsx             # 全局Provider（如果有）

  features/                   # 核心：按功能垂直切分，每个 feature 自包含
    dashboard/
      components/
        StatsCards.tsx            # 累计验证数/有效率/Catch-all域名数/今日验证数 四个统计卡片
        QuickVerifyCard.tsx       # 复用 single-verify 的表单组件，紧凑模式渲染
        RecentActivityTable.tsx   # 最近10条历史记录只读展示
      hooks/
        useDashboardStats.ts      # 调用 get_dashboard_stats command
      api/
        getDashboardStats.ts
      types.ts
      index.ts

    single-verify/
      components/
        SingleVerifyForm.tsx  # TanStack Form 输入框
        ResultCard.tsx        # 单个验证结果展示卡片
      hooks/
        useSingleVerify.ts    # 封装 invoke('verify_single_email', ...) 的调用逻辑
      api/
        verifySingleEmail.ts  # 纯粹的 Tauri command 调用封装，不掺业务逻辑
      types.ts
      index.ts                # 该feature对外导出的公共接口（barrel file）

    batch-verify/
      components/
        BatchInputPanel.tsx       # Textarea粘贴 + 文件导入
        BatchResultsTable.tsx     # TanStack Table 渲染
        BatchProgressBar.tsx      # shadcn Progress
      hooks/
        useBatchVerify.ts         # 发起/取消批量任务，监听Tauri进度事件
      api/
        verifyBatchEmails.ts
      store/
        batchStore.ts             # Zustand：batchStatus/batchProgress/batchResults 只属于这个feature
      columns.tsx                 # TanStack Table 的列定义单独拆出来，方便复用到历史记录页
      types.ts
      index.ts

    history/
      components/
        HistoryTable.tsx
      hooks/
        useHistory.ts             # 用 tauri-plugin-sql 查历史记录
      api/
        fetchHistory.ts
      types.ts
      index.ts

    settings/
      components/
        SettingsForm.tsx          # TanStack Form
      hooks/
        useSettings.ts
      store/
        settingsStore.ts          # Zustand：heloDomain/timeout/cooldown/concurrency，配合 tauri-plugin-store 持久化
      types.ts
      index.ts

  shared/                     # 只放真正跨多个feature复用的东西，宁缺毋滥
    components/
      ui/                       # shadcn/ui 生成的组件默认输出到这里（button.tsx、card.tsx等）
    lib/
      tauri.ts                  # invoke/listen 的通用封装、错误处理
      utils.ts
    types/
      verify-result.ts          # VerifyResult / Verdict —— 这两个类型 single-verify 和 batch-verify 都要用，
                                 # 提到 shared 里，避免两边各自重复定义

  routes/                     # TanStack Router 文件式路由，只做"页面组装"，不写业务逻辑
    index.tsx                 # "/" —— 引用 features/dashboard 的组件（Dashboard总览，首页）
    single.tsx                 # "/single" —— 引用 features/single-verify 的组件
    batch.tsx                 # "/batch" —— 引用 features/batch-verify 的组件
    history.tsx                # "/history" —— 引用 features/history 的组件
    settings.tsx               # "/settings" —— 引用 features/settings 的组件
```

几条组织原则：

- **`routes/` 保持"薄"**：路由文件只负责把某个 feature 的顶层组件拼进页面布局，不写具体的业务逻辑/状态管理，方便以后要调整路由结构（比如把某个页面挪到嵌套路由下）时不用动 feature 内部代码。
- **跨 feature 复用要经过 `shared/`，不要 feature 之间互相 import 内部文件**：如果 `batch-verify` 需要用到 `settings` 里的并发数配置，应该通过 `features/settings/index.ts` 导出的公共 hook（比如 `useSettings()`）去读，而不是直接 `import` `settings/store/settingsStore.ts` 内部实现，保持每个 feature 的封装边界。
- **每个 feature 的 Zustand store 独立**，不要走"全局大 store 一个文件管所有状态"的老路子——`batchStore` 只管批量验证相关状态，`settingsStore` 只管设置项，两者需要交叉读取时通过对方 feature 暴露的 hook 来读，不直接跨 store 耦合内部字段。
- **`index.ts` barrel file 控制 feature 的对外可见面**：只导出这个 feature 真正需要被外部（routes或其他feature）使用的组件/hook/类型，内部实现细节（比如某个私有子组件）不导出，减少无意间产生的耦合。

### 0.1 路由结构（TanStack Router）

```
/                  → Dashboard 总览页（首页，对应 features/dashboard）
/single            → 单邮箱验证页（对应 features/single-verify）
/batch             → 批量验证页（对应 features/batch-verify）
/history           → 历史记录/缓存页（对应 features/history）
/settings          → 设置页（对应 features/settings）
```
建议用 TanStack Router 的文件式路由（`src/routes/` 目录下按文件自动生成路由树），配合 `@tanstack/router-devtools` 在开发环境调试路由状态。侧边用 shadcn 官方的 `Sidebar` 组件族做导航（collapse模式选`icon`），具体的 Sidebar 结构、每个页面的详细UI/交互设计见本文档第八节《界面 UI / UX 设计规范》。

### 0.2 状态设计（Zustand，按 feature 拆分，而不是一个全局大 store）

呼应上面 feature-based 的组织原则，**不要**建一个笼统的 `useVerifyStore` 把批量验证状态和设置项状态揉在一起，而是拆成两个各自归属对应 feature 的 store：

```ts
// features/batch-verify/store/batchStore.ts
interface BatchStore {
  batchStatus: 'idle' | 'running' | 'cancelling' | 'done';
  batchProgress: { completed: number; total: number };
  batchResults: VerifyResult[];   // 从 shared/types/verify-result.ts 引入

  startBatch: (emails: string[]) => Promise<void>;
  cancelBatch: () => Promise<void>;
}
```

```ts
// features/settings/store/settingsStore.ts
interface SettingsStore {
  // 持久化优先用 tauri-plugin-store（官方插件，跨平台写到应用数据目录的json文件），
  // 不要用 zustand 的 persist middleware 存 localStorage：Tauri webview 的 localStorage 在不同系统/打包环境下
  // 行为不总是稳定，plugin-store 是更可靠的官方方案，Zustand 这里只做内存态的镜像，方便组件读取
  heloDomain: string;
  smtpTimeoutSeconds: number;
  domainCooldownSeconds: number;
  maxConcurrentDomains: number;

  updateSettings: (partial: Partial<Omit<SettingsStore, 'updateSettings'>>) => void;
}
```

`batch-verify` 需要读并发数/超时这些设置值时，通过 `features/settings/index.ts` 导出的 `useSettings()` hook 去读（内部就是包了一层 `settingsStore`），不要在 `batchStore.ts` 里直接 `import` `settingsStore` 内部实现，保持两个 feature 的边界清晰。

批量验证发起后，前端监听 Tauri 事件（`listen('verify-progress', ...)`），每收到一条后端 emit 的进度事件，就调用 `batchStore` 的 `set()` 更新 `batchProgress` 和 `batchResults`，让 UI 实时刷新，不需要轮询。

### 1. 单邮箱验证页

- 用 TanStack Form 管理输入框的表单状态（值、是否已提交、loading等UI层面的状态），**不在前端做邮箱格式的正则校验**——按"核心原则"，语法校验也是 Rust 的职责，直接调 `verify_single_email` command，由后端返回的 `syntax_valid` 字段驱动 UI 展示对错，前端表单层面只做"是否为空"这种纯交互性质的禁用提交按钮判断，不重复实现一份格式校验规则
- shadcn 的 `Input` + `Button`（loading 状态用 `Button` 的 disabled + 一个 `Loader2` 图标转圈）
- 验证过程中显示 loading 状态（因为 SMTP 握手有网络延迟，通常 1-5 秒）
- 结果用 shadcn 的 `Card` 展示，参考 verifyemailaddress.org 那种风格：
  - 顶部大字：邮箱地址 + 是否有效的醒目标识（用 shadcn 的 `Badge` 组件，绿色✅valid/红色❌invalid/黄色⚠️unknown或catch-all三种变体）
  - 下方分点列出：语法校验、MX 记录、SMTP 响应码、catch-all 提示、原始服务器返回消息

### 2. 批量验证页

- 支持两种输入方式：
  - 文本框（shadcn `Textarea`）直接粘贴（每行一个邮箱）
  - 拖拽/选择 CSV 或 TXT 文件导入（用 Tauri 的文件对话框 API `@tauri-apps/plugin-dialog`）
- 开始验证后显示：
  - 总体进度条（shadcn `Progress` 组件，绑定 Zustand 里的 `batchProgress`）
  - **实时结果表格用 TanStack Table 实现**：
    - 列定义：邮箱 | 状态(verdict，用彩色Badge渲染) | SMTP响应码 | catch-all(是/否/-) | 耗时 | 原始服务器消息
    - 开启 TanStack Table 的排序功能（点表头按状态/耗时排序）
    - 开启筛选功能（表格上方加个 shadcn `Select` 或 `ToggleGroup`，筛选只看 valid / invalid / unknown / catch-all）
    - 数据量大（比如几千个邮箱）时开启分页（TanStack Table 的 `getPaginationRowModel`），避免一次性渲染卡顿
  - "停止"按钮（shadcn `Button` variant="destructive"），点击调用 `cancelBatch()`，中途取消剩余未验证的邮箱
- 验证完成后：
  - 汇总统计卡片（有效/无效/未知/catch-all风险 各多少个及占比，直接渲染 `verify_batch_emails` command 返回的 `BatchSummary`，前端不用自己对结果数组做`.filter().length`重新算一遍）
  - "导出CSV"按钮，把当前筛选后（或全部）的邮箱列表传给 `export_results_to_csv` command，由 Rust 侧的 `csv` crate 生成文件内容并通过 `tauri-plugin-fs` 写盘，前端只负责收集"要导出哪些行"这个交互意图并调用 command，不在前端拼CSV字符串

### 3. 历史记录/缓存页（可选，加分项）

- 同样用 TanStack Table 展示之前验证过的邮箱和结果，数据来源是调用 `fetch_history` command（Rust 内部用 `tauri-plugin-sql` 查 SQLite），**不直接用插件的前端JS API发SELECT查询**，域名筛选这个查询条件通过 command 参数传给后端拼SQL，前端只是把用户选的筛选值传过去
- 表格上的域名筛选下拉框只是UI交互，实际的筛选逻辑（拼WHERE条件）在 Rust 里做
- 可以加一个"重新验证"按钮，对历史记录里的某一行重新触发单次验证

### 4. 设置页

- 用 TanStack Form 构建整个设置表单，对数值型字段（超时时间、冷却间隔、并发数）做范围校验（比如超时不能设成0或负数，并发数建议限制在1-20之间避免用户手滑设置过大导致被目标邮件服务器判定滥用）
- 允许用户自定义：
  - HELO 域名（发起探测时用什么域名自报家门）
  - SMTP 超时时间
  - 域名探测冷却间隔
  - 批量验证的并发域名数
- 表单项用 shadcn 的 `Form`（配合 TanStack Form 的 field 绑定）+ `Input`/`Slider` 组件；数值范围校验（超时/并发数的合法区间判断）放在 Rust 侧的 `update_settings` command 里做，前端表单只负责收集输入值，保存时调用 `update_settings` command（内部才是真正调 `tauri-plugin-store` 写磁盘），应用启动时调用 `get_settings` command 读回来初始化 `settingsStore`，前端不直接触碰 `tauri-plugin-store` 的API

## 五、边界情况处理

- **网络层面完全连不上 25 端口**：应用启动时做一次自检（尝试连接一个已知稳定的 MX 节点，比如 Gmail 的），如果失败要在 UI 上明显提示"当前网络无法进行 SMTP 探测，请检查网络环境（很多家庭宽带/公司网络会封锁出站25端口）"，而不是让用户批量验证的时候才发现全部失败
- **灰名单（451临时拒绝）**：标记为 unknown，并在结果里注明"建议几分钟后重试"，批量任务里可以自动做一次延迟重试（比如等30秒后对这批 unknown 结果重新探测一次）
- **超时**：给每次 SMTP 握手设置硬超时（8-10秒），避免一个卡住的连接拖慢整个批量任务
- **重复邮箱去重**：批量输入前先对邮箱列表去重，避免浪费探测次数
- **域名分组优化**：批量验证前先按 `@`后面的域名分组，同域名共享一次 catch-all 判定结果，不用每个邮箱都重新测一次 catch-all

## 六、打包与分发

- `tauri build` 产出对应平台的安装包
- 注意 Rust 的网络权限在不同操作系统上的差异：macOS 可能需要处理网络访问权限弹窗；Windows 防火墙首次运行可能会弹出询问是否允许该程序访问网络，需要在文档里提前告知用户点"允许"

## 七、合规提醒（务必在应用内提示用户）

- 批量高频对同一批目标邮件服务器做 SMTP 探测，有被目标服务器判定为滥用行为、进而拉黑你所在网络出口 IP 的风险
- 请自行确认使用场景是否符合当地及目标邮箱服务商的相关法规（如 GDPR、CAN-SPAM 等），批量验证他人邮箱涉及个人数据处理，建议仅用于验证你已通过合法渠道（如官网公开信息、名片、对方主动提供）获取的邮箱，不用于未经同意的大规模数据收集场景

## 八、界面 UI / UX 设计规范

> 定义整个应用的界面结构、每个页面的布局、以及每个动作/交互的具体行为。

### 一、整体结构：Sidebar + 内容区

采用经典的"左侧固定导航 + 右侧内容区"桌面应用布局，用 shadcn/ui 官方的 `Sidebar` 组件族实现（`SidebarProvider` / `Sidebar` / `SidebarContent` / `SidebarMenu` / `SidebarTrigger`），collapse 模式选 **`icon`**（收起后变成只显示图标的窄条，而不是完全滑出屏幕的 `offcanvas` 模式——桌面应用场景下用户希望导航栏一直可见，只是节省空间）。

#### Sidebar 结构（从上到下）

```
┌─────────────────────────┐
│ [Logo] SKY FLUX VERIFY   │ ← Header区：应用名/图标，收起状态下只显示图标
├─────────────────────────┤
│ 🏠 Dashboard             │ ← 导航菜单组
│ ✉️  单邮箱验证            │
│ 📋 批量验证              │
│ 🕐 历史记录              │
├─────────────────────────┤
│ ⚙️  设置                 │ ← 单独分组，视觉上跟主导航隔开（用SidebarGroup分组+分隔线）
├─────────────────────────┤
│ 🟢 网络就绪  /  🔴 25端口不可用 │ ← Footer区：常驻网络健康状态指示器
│ [«] 收起侧边栏            │ ← 折叠/展开按钮，快捷键 Ctrl/Cmd+B
└─────────────────────────┘
```

**导航项交互**：
- 当前所在页面对应的菜单项高亮（背景色+左侧竖条强调色，shadcn Sidebar 的 `isActive` 状态）
- 鼠标悬停非当前项：背景浅色高亮，光标变pointer
- 点击：用 TanStack Router 的 `<Link>` 做客户端路由跳转，无整页刷新
- 收起状态（icon-only）下，悬停图标显示 Tooltip 展示完整文字（"批量验证"等）
- 折叠/展开按钮：点击后 Sidebar 用 CSS transition（200ms ease）平滑收缩宽度，图标本身不做旋转动画，只是文字部分做淡出

**网络状态指示器**（Footer区，跨所有页面常驻）：
- 应用启动时自动跑一次 25 端口连通性自检（对应之前讨论过的"应用启动自检"逻辑，调用Rust侧的 `check_network_health` command）
- 绿色圆点 + "网络就绪"：可以正常做SMTP探测
- 红色圆点 + "25端口不可用"：点击后弹出 shadcn `HoverCard` 或 `Popover`，展示"当前网络可能封锁了出站25端口，很多家庭宽带/公司网络会有这个限制"的说明文字 + 一个"重新检测"按钮
- 黄色圆点（可选第三种状态）+ "检测中..."：应用刚启动、自检还没跑完时的过渡态

### 二、路由结构（更新版）

```
/            → Dashboard（总览，首页）
/single      → 单邮箱验证
/batch       → 批量验证
/history     → 历史记录
/settings    → 设置
```

（对应 `tauri_app_spec_prompt.md` 里之前把单邮箱验证放在 `/` 的设计，现在调整为 `/` 是独立的 Dashboard 总览页，单邮箱验证挪到 `/single`。）

---

### 三、页面详细设计

#### 1. Dashboard（`/`）

**用途**：打开应用第一眼看到的总览页——关键数字一目了然、能立刻做一次快速验证、能看到最近的活动，并引导用户去开始批量验证。

**布局（从上到下）**：

1. **顶部标题栏**："Dashboard" + 右侧网络状态Badge（跟Sidebar footer的状态联动，这里再展示一次是为了内容区第一屏就能看到，不用去看sidebar）

2. **统计卡片行**（4个 shadcn `Card` 横向排列，响应式：窄窗口自动换行）：
   - 「累计验证数量」+ 大数字
   - 「整体有效率」+ 百分比（valid数 / 总数）
   - 「Catch-all 域名数」+ 数字（提醒用户这些域名的历史结果不可信）
   - 「今日验证数量」+ 数字（当天0点以来）
   - 数据来源：一个新增的 Rust command `get_dashboard_stats()`，内部对 SQLite 的 `verify_results` 表做聚合查询（COUNT/AVG等），前端只渲染数字，不在前端对历史数组做reduce计算

3. **快速验证卡片**（一个突出的 `Card`，标题"快速验证一个邮箱"）：
   - 单行 Input + "验证"按钮（跟 `/single` 页面用的是同一个 `features/single-verify` 组件，只是以紧凑模式渲染，复用不重复实现）
   - 提交后不跳转页面，在这张卡片内部展开一个简化版结果条（一行文字：邮箱+Badge+SMTP响应码），带一个"查看完整详情"链接跳转到 `/single`（并把这次的结果通过路由 state 带过去，避免重新验证一次）

4. **最近活动表格**（一个 `Card`，标题"最近验证"+右上角"查看全部"链接）：
   - 展示最近10条历史记录（`fetch_history` 加 limit 参数），只读展示，不带排序筛选交互（要交互去 `/history`）
   - 空状态：如果一条记录都没有，这张卡片显示"暂无验证记录"+ 一个引导按钮"去验证第一个邮箱"

5. **底部大按钮**："开始批量验证" —— 醒目的主色按钮，点击跳转 `/batch`

**交互细节**：
- 页面挂载（`useEffect`/路由的`loader`）时并行请求：`get_dashboard_stats`、`fetch_history(limit=10)`、网络自检状态（如果sidebar已经测过，直接读共享状态不重复测）
- 统计卡片数字加载中显示 shadcn `Skeleton` 占位，不要显示"0"再跳到真实值（避免误导用户以为真的是0）
- 统计卡片支持简单的 hover 提示（`Tooltip`），显示更精确的数值来源说明，比如"整体有效率"hover显示"基于全部历史验证记录计算"

---

#### 2. 单邮箱验证（`/single`）

**用途**：需要看完整诊断信息时用这个页面（Dashboard的快速验证只给摘要）。

**布局**：

1. 页面标题"单邮箱验证"
2. 输入区：TanStack Form 管理的 `Input`（placeholder: "输入要验证的邮箱地址"）+ "验证"`Button`
   - 按钮在输入为空时 `disabled`（这是唯一允许在前端做的"校验"——纯粹的空值判断，不涉及邮箱格式规则）
3. 结果区（三态）：
   - **idle**：居中的提示文案 + 图标，"输入邮箱地址开始验证"
   - **loading**：一张骨架屏 `Skeleton`，形状跟真实结果卡片一致（避免布局跳动），按钮内 `Loader2` 图标旋转+文字变"验证中..."
   - **result**：`Card` 展示——
     - 顶部：邮箱地址（大字）+ 状态 `Badge`（见下方"Badge规范"）
     - 中间：`Separator` 分隔的信息列表——语法校验（✓/✗图标+文字）、MX记录（列出解析到的host，多个用换行展示）、SMTP响应码（数字+原始message文本，等宽字体展示原文）、Catch-all判定（是/否/不适用）、验证耗时（毫秒）
     - 底部按钮组：「复制结果」「重新验证」「清空」
   - **error**（Rust command 返回 Err）：`Alert` 组件（destructive变体），展示 `AppError` 的具体分类文案（比如"DNS查询失败"而不是笼统的"出错了"）

**Badge 规范**（贯穿整个应用，历史记录页、批量结果表格都复用同一套）：
| Verdict | 颜色 | 文案 |
|---|---|---|
| Valid | 绿色（`bg-green-100 text-green-700` 或 shadcn的`default`success变体） | 有效 |
| Invalid | 红色（destructive变体） | 无效 |
| Unknown | 黄色（warning，shadcn默认没有这个变体需要自定义一个） | 未知，建议稍后重试 |
| RiskyCatchAll | 橙色（自定义变体） | Catch-all，结果不可信 |

**交互细节**：
- 表单提交：Enter 键或点击按钮均可触发，两者调用同一个 handler
- 「复制结果」：调用 `tauri-plugin-clipboard-manager` 把结果格式化成一段文字复制到剪贴板，按钮文字短暂变成"已复制 ✓"（2秒后恢复原文字），不用额外弹Toast（按钮自身的状态变化已经是足够的反馈）
- 「重新验证」：不清空输入框，直接用当前邮箱重新调一次 command，走一遍 loading→result
- 「清空」：输入框和结果区都清空，回到 idle 状态
- 结果卡片出现时用简单的 CSS `transition`（opacity+translateY，200ms）做淡入，不引入额外动画库

---

#### 3. 批量验证（`/batch`）

**用途**：核心功能页，同一个路由内通过状态机切换三种视图，不做路由跳转。

**状态机**：`idle`（输入）→ `running`（验证中）→ `done`（完成）；`done` 状态下可以点"重新开始一批"回到 `idle`。

#### 3.1 idle 态布局

1. 页面标题"批量验证"
2. `Tabs` 组件，两个 tab："粘贴文本" / "导入文件"
   - **粘贴文本 tab**：大 `Textarea`（placeholder"每行一个邮箱地址"），下方实时显示"检测到 N 个邮箱地址（已自动去重）"——注意：这里的"去重计数"是纯粹的字符串处理展示，不涉及邮箱格式判断，可以前端做（按行分割+trim+Set去重计数），因为这不是"验证逻辑"而是"输入回显"性质的交互反馈；真正决定这些地址是否合法/有效仍然全部交给Rust
   - **导入文件 tab**：虚线边框的拖拽区域（`Card` + dashed border），文案"拖拽 CSV/TXT 文件到这里，或点击选择文件"；拖拽悬停时边框变实线+背景浅色高亮；点击"选择文件"调用 `tauri-plugin-dialog` 的文件选择对话框；文件选中/拖入后，调用Rust侧的解析command（不在前端parse CSV），解析完把邮箱列表回填展示在同一个Textarea里（切回"粘贴文本"tab展示，让用户在提交前还能手动增删）
3. 底部"开始验证"大按钮，邮箱列表为空时 disabled

#### 3.2 running 态布局

1. 顶部替换成：`Progress` 进度条 + 文字"已完成 23 / 100"，右侧"停止"按钮（destructive variant）
2. 下方 TanStack Table 实时结果表格：
   - 列：邮箱 | 状态（Badge）| SMTP响应码 | Catch-all | 耗时(ms) | 原始消息
   - 每完成一个邮箱，通过Tauri事件推送，新增的行短暂高亮背景（比如浅绿色闪烁一次，800ms后恢复），提示用户"这行是刚更新的"
   - 表格上方筛选 `ToggleGroup`：全部/有效/无效/未知/Catch-all风险（运行中也可以筛选查看已完成的部分）
   - 排序：点击"状态"或"耗时"表头

**交互**：
- 点击"停止"：先弹 `AlertDialog` 二次确认（"确定要停止吗？已完成的验证结果会保留"），确认后调用 `cancel_batch_verification`，按钮变灰+文字"正在停止..."，直到后端确认停止完成才切换到 `done` 态（此时done态里只有已完成的部分）

#### 3.3 done 态布局

1. 顶部替换成汇总统计（4个小 `Card` 横排：有效/无效/未知/Catch-all风险，数字+百分比，直接渲染 `BatchSummary`，不在前端重新计算）
2. 保留同一张结果表格（此时数据不再增长，支持完整的排序/筛选/分页——大批量时开分页，`getPaginationRowModel`）
3. 操作按钮组：「导出CSV（当前筛选）」「导出CSV（全部结果）」「重新开始一批新的」

**交互**：
- 「导出CSV」：调用 `tauri-plugin-dialog` 的保存对话框选路径 → 调用 `export_results_to_csv` command → 成功后 Toast 提示"已导出到 [路径]"，失败显示错误 Toast
- 「重新开始一批新的」：状态机回到 `idle`，清空Textarea和结果表格（这里给一个二次确认，因为完成的结果如果没导出会丢失展示——不过数据本身已经存进历史记录了，可以在确认弹窗文案里提一句"结果已保存到历史记录，可以在'历史记录'页面找回"，减轻用户的操作焦虑）
- 批量完成瞬间（`running`→`done`的切换时刻）：如果应用窗口当前不在前台/被最小化，触发 `tauri-plugin-notification` 系统通知；如果窗口可见，用一个轻量的 `Toast`（shadcn的Sonner）提示"批量验证已完成"，不強制打断用户

---

#### 4. 历史记录（`/history`）

**用途**：浏览所有历史验证记录，支持筛选和对单条记录重新验证。

**布局**：

1. 页面标题"历史记录"
2. 筛选栏（横向排列）：
   - 域名筛选 `Select`（选项来自Rust侧对历史表做`SELECT DISTINCT domain`查询返回，不在前端对全部记录做去重取域名）
   - 邮箱模糊搜索 `Input`（带搜索图标，debounce 300ms后触发查询，同样是传参给`fetch_history`由Rust做`LIKE`查询，不是前端过滤已加载的数组）
   - 「重置筛选」按钮
3. TanStack Table：
   - 列：邮箱 | 验证时间 | 状态（Badge）| SMTP响应码 | Catch-all | 操作
   - 操作列：一个"重新验证"图标按钮（`RefreshCw` icon）
   - 底部分页控件："共 N 条，第 X / Y 页"
4. 空状态（无任何记录时）：居中图标+文案"还没有验证记录"+ 按钮"去验证一个邮箱"（跳转`/single`）/"去批量验证"（跳转`/batch`）

**交互**：
- 域名 `Select` 选择后立即重新查询（不需要额外点"搜索"按钮），查询期间表格区域显示轻量的loading遮罩（不是整页loading，避免筛选栏跟着一起闪烁）
- 点击"重新验证"图标：该行的Badge位置临时替换成一个小的旋转loading图标，调用`verify_single_email`，完成后只更新这一行的数据（不刷新整个表格/不丢失当前筛选和分页状态）
- 点击表格行（非操作列区域）：从右侧滑出一个 `Sheet` 抽屉，展示这条记录的完整字段（包括表格里没展示的：完整MX记录列表、SMTP原始message全文、UUID等），抽屉里也有一个"重新验证"按钮

---

#### 5. 设置（`/settings`）

**用途**：管理探测参数和应用相关配置。

**布局**：

1. 页面标题"设置"
2. 分组卡片：
   - **探测参数**（`Card`）：
     - HELO 域名（`Input`，文本）
     - SMTP 超时时间（`Input` type=number，单位秒）
     - 域名探测冷却间隔（`Slider`，单位秒，范围建议1-10）
     - 最大并发域名数（`Slider`，范围1-20，超过20会有警示文案"设置过高容易被目标邮件服务器判定滥用"常驻显示在Slider下方，不是等用户拖到头才提示)
   - **应用信息**（`Card`）：
     - 版本号只读展示
     - "检查更新"按钮（如果接入了`tauri-plugin-updater`）
     - "打开数据目录"按钮——用 `tauri-plugin-opener`（Tauri v2 里 `shell.open` 的官方替代插件）在系统文件管理器里定位到SQLite数据库文件所在目录，方便用户手动备份
3. 页面/表单底部固定"保存"按钮（sticky在视口底部，滚动时始终可见）

**交互**：
- 不做逐字段 `onBlur` 实时校验（这么做要么得在前端重复实现一份校验规则、要么每次失焦都调一次command体验很差，两者都不理想）。改成：所有字段自由输入，点击"保存"时统一调用 `update_settings` command，Rust侧做范围校验，如果某个字段不合法，返回的 `AppError` 里带字段标识，前端只需要把这个错误信息显示在对应字段下方（红色小字），不自己判断"合法与否"
- "保存"点击：按钮进入loading态，成功后 Toast"设置已保存"，同时更新 `settingsStore`；失败则不更新store，错误信息展示在对应字段下
- "打开数据目录"点击：调用command，如果失败（比如目录不存在）用Toast提示错误

---

### 四、贯穿全局的交互规范

- **Toast通知**：统一用 shadcn 集成的 Sonner，成功用默认样式，失败用destructive样式，位置固定右下角，自动消失时间4秒（重要错误可以设置不自动消失，需要用户手动关闭）
- **二次确认**：所有"不可逆/有代价"的操作（停止批量任务、重新开始清空当前批次）都过 `AlertDialog`，普通的导航/查看类操作不需要确认
- **键盘可达性**：所有可交互元素支持Tab键导航，主要操作（验证按钮、保存按钮）支持Enter键触发，Sidebar折叠支持`Ctrl/Cmd+B`快捷键
- **加载态**：任何等待Rust command返回的地方，不允许"无反馈的静止等待"——按钮要么disabled+spinner，要么对应区域要有Skeleton或loading指示，杜绝用户以为"卡死了"
- **颜色语义一致性**：绿色=有效/成功，红色=无效/危险操作，黄色=需要注意/未知，橙色=catch-all这种"技术上通过但不可信"的特殊状态，全局统一，不同页面不能有歧义

---

## 交给 AI 编码工具时可以直接使用的一句话摘要

> 项目名 **SKY FLUX VERIFY**（脚手架目录名 `verify`），identifier 为 `com.sky-flux.verify`。**核心原则：能用Rust解决的逻辑全部放Rust，前端只负责UI渲染和交互**——邮箱语法校验、MX查询、SMTP探测、catch-all判定、限流冷却、批量调度、CSV解析导出、汇总统计全部在Rust后端完成，前端通过Tauri command拿到算好的最终结果直接渲染，不重复实现任何一份业务逻辑（包括前端不直接调用tauri-plugin-sql/tauri-plugin-store的JS API，一律封装成Rust command）。前端代码按 feature-based 方式组织（`features/single-verify`、`features/batch-verify`、`features/history`、`features/settings` 各自独立，只有真正跨feature复用的东西放 `shared/`，`routes/` 只做页面组装，Zustand store 按feature拆分）。SQLite 所有表主键统一用 **UUID v7**（`uuid` crate开`v7` feature，`Uuid::now_v7()`生成，存TEXT类型），不用自增整数或UUID v4。用 Tauri v2 + Rust 后端，前端用 React 19 + TypeScript + shadcn/ui（底层 primitive 用 Base UI，不用 Radix）+ Zustand + TanStack Router + TanStack Form + TanStack Table + Biome，开发一个跨平台桌面邮箱验证工具。持久化、对话框、文件读写、系统通知、日志这些能力一律优先用官方 Tauri 插件（`tauri-plugin-sql`/`tauri-plugin-store`/`tauri-plugin-dialog`/`tauri-plugin-fs`/`tauri-plugin-notification`/`tauri-plugin-log`），不自己额外引入第三方 crate 重复造轮子。核心是 Rust 侧手写 SMTP 协议握手（EHLO→MAIL FROM→RCPT TO→QUIT，不发送DATA）来判断邮箱是否存在，支持单个验证和批量验证（Textarea粘贴或CSV导入，TanStack Table展示结果支持排序筛选分页，Zustand管理批量进度状态并监听Tauri事件实时更新），批量验证要按域名分组做并发控制和探测冷却，支持 catch-all 域名识别、结果缓存到本地SQLite、历史记录页、导出CSV、批量完成后弹系统通知、实时进度展示和取消功能，代码规范用Biome。路由结构为 `/`(Dashboard总览)、`/single`(单邮箱验证)、`/batch`(批量验证)、`/history`(历史记录)、`/settings`(设置)，左侧用shadcn官方Sidebar组件（icon收起模式）做导航。**`shared/components/ui/`下shadcn生成的组件源码不允许手动修改，样式覆盖用`cn()`函数传className，功能不够时用多个shadcn组件组合封装成新的业务组件放在对应feature目录下，优先直接用shadcn现成组件，不自己从零写UI组件。**参考本文档第二、三、四节的技术栈、官方插件清单、模块划分和数据结构实现，具体每个页面的UI布局和交互细节见本文档第八节。

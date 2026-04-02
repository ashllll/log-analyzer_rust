# 贡献指南

本指南面向直接修改仓库代码的开发者，目标是让提交保持可验证、可回滚、可维护。

---

## 开发环境

### 系统要求

| 工具 | 版本要求 | 说明 |
|------|---------|------|
| Node.js | >= 22.12.0 | 前端运行时 |
| npm | >= 10 | 包管理器 |
| Rust | >= 1.70 | Rust 工具链 |
| Tauri 前置依赖 | 平台相关 | 见下方说明 |

### Tauri 平台前置依赖

**macOS：**

```bash
xcode-select --install
```

**Linux（Ubuntu/Debian）：**

```bash
sudo apt update
sudo apt install libwebkit2gtk-4.1-dev build-essential curl wget \
  file libxdo-dev libssl-dev libayatana-appindicator3-dev librsvg2-dev
```

**Windows：**

安装 Visual Studio Build Tools（含 C++ 桌面开发工作负载）和 WebView2。

详见 [Tauri 2 官方前置依赖文档](https://v2.tauri.app/start/prerequisites/)。

### 初始化项目

```bash
git clone https://github.com/ashllll/log-analyzer_rust.git
cd log-analyzer_rust/log-analyzer
npm install
```

---

## 目录约定

| 路径 | 说明 |
|------|------|
| `log-analyzer/src/` | React 前端（页面、组件、Hook、状态） |
| `log-analyzer/src-tauri/src/` | Tauri 后端主 crate（命令、服务、存储） |
| `log-analyzer/src-tauri/crates/la-core/` | 公共错误类型、领域模型、抽象 Trait |
| `log-analyzer/src-tauri/crates/la-storage/` | CAS 对象存储与 SQLite 元数据实现 |
| `log-analyzer/src-tauri/crates/la-search/` | 搜索结果分页存储、Tantivy 索引基础设施 |
| `log-analyzer/src-tauri/crates/la-archive/` | ZIP/TAR/GZ/RAR/7Z 归档处理核心逻辑 |
| `docs/` | 长期维护的核心文档 |
| `scripts/` | CI 校验脚本 |

---

## 开发与构建命令

### 前端脚本

| 命令 | 说明 |
|------|------|
| `npm run dev` | 启动 Vite 开发服务器（纯前端） |
| `npm run build` | TypeScript 编译 + Vite 生产构建 |
| `npm run tauri dev` | 启动 Tauri 开发模式（前端 + 后端热重载） |
| `npm run tauri build` | Tauri 生产构建（含平台安装包） |
| `npm run type-check` | TypeScript 类型检查（无输出为通过） |
| `npm run lint` | ESLint 检查 |
| `npm run lint:fix` | ESLint 自动修复 |
| `npm test` | 运行 Jest 测试套件 |
| `npm run test:watch` | Jest 监听模式（开发时使用） |
| `npm run validate:ci` | 本地 CI 完整校验脚本 |
| `npm run preview` | Vite 生产构建本地预览 |

### Rust 命令

| 命令 | 说明 |
|------|------|
| `cargo fmt` | 格式化代码 |
| `cargo fmt -- --check` | 格式检查（不修改，CI 中使用） |
| `cargo clippy --all-features --all-targets -- -D warnings` | Lint 检查（warnings 视为错误） |
| `cargo test -q` | 运行全量测试（包含所有 workspace crate） |
| `cargo test -q <test_name>` | 运行指定测试 |
| `cargo build` | 编译检查（debug 模式） |
| `cargo build --release` | 生产构建 |

---

## 提交前最少验证

**前端（在 `log-analyzer/` 下执行）：**

```bash
npm run lint
npm run type-check
npm test
```

**Rust（在 `log-analyzer/src-tauri/` 下执行）：**

```bash
cargo fmt -- --check
cargo clippy --all-features --all-targets -- -D warnings
cargo test -q
```

若改动只触及单一模块，至少运行对应模块的测试：

```bash
# 只跑 la-storage 的测试
cargo test -q -p la-storage

# 只跑 search 相关测试
cargo test -q search
```

---

## 搜索链路约定

在修改搜索相关代码前，必须先明确当前主链路：

**当前主搜索链路：**

```text
前端入口: src/pages/SearchPage.tsx
后端入口: src-tauri/src/commands/search.rs: search_logs()
核心匹配: services/query_executor.rs + services/query_planner.rs
分页读取: search_engine/disk_result_store.rs + fetch_search_page
```

**注意事项：**

- UI 主搜索使用 `|` 分隔的简单字符串查询（OR 逻辑）
- `execute_structured_query` 是独立命令，不是 UI 搜索主路径
- Tantivy 索引在导入时建立，但当前主搜索仍走 CAS 文件扫描路径
- 不要把 `FilterEngine` / `TimePartitionedIndex` 等预留能力误当成主搜索执行器

---

## 归档提取策略配置

归档解压行为通过 TOML 配置文件控制：

- 模板：`src-tauri/config/extraction_policy.toml.example`
- 运行时路径：`src-tauri/config/extraction_policy.toml`（已在 `.gitignore` 中）

| 配置段 | 参数 | 说明 |
|--------|------|------|
| `[extraction]` | `max_depth` | 嵌套归档最大深度（1-20，默认 10） |
| `[extraction]` | `max_file_size` | 单文件大小上限（字节） |
| `[extraction]` | `max_total_size` | 单归档总大小上限 |
| `[extraction]` | `use_enhanced_extraction` | 启用高级特性（长路径、zip 炸弹检测等） |
| `[security]` | `enable_zip_bomb_detection` | 启用 zip 炸弹检测（默认 true） |
| `[security]` | `compression_ratio_threshold` | 压缩比阈值（默认 100.0） |
| `[paths]` | `enable_long_paths` | Windows 长路径支持 |
| `[performance]` | `enable_streaming` | 流式解压（降低内存占用） |
| `[audit]` | `enable_audit_logging` | 审计日志 |

---

## 文档维护规则

- 代码行为变化时，必须同步更新对应文档
- 只保留长期需要维护的文档（历史一次性分析报告不要留在 `docs/`）
- 文档描述必须以当前代码**真实行为**为准，不把预留能力写成已投入主链路
- 修改搜索命令的参数或返回值时，必须同步更新：
  - `src/services/api.ts`
  - 对应前端页面组件
  - `docs/architecture/API.md`

---

## 提交流程

推荐步骤：

1. 在改动前确认真实业务路径（不要修改未使用的预留代码）
2. 先补测试或验证用例，再修改实现
3. 运行最少验证集（见上方）
4. 更新受影响的文档
5. `git diff` 自查后再提交

**提交信息格式（Conventional Commits）：**

```text
<type>(<scope>): <description>

<optional body>
```

常用 type：

| type | 说明 |
|------|------|
| `feat` | 新功能 |
| `fix` | Bug 修复 |
| `refactor` | 重构（不改变外部行为） |
| `docs` | 文档变更 |
| `test` | 测试相关 |
| `chore` | 构建/CI/依赖更新 |
| `perf` | 性能优化 |
| `ci` | CI 流水线变更 |

**示例：**

```text
fix(search): precompile filters before scan to avoid per-line reparse

feat(archive): add checkpoint support for nested extraction
```

---

## 评审关注点

提交评审时优先检查：

- 是否修改了真实主链路，而不是未启用的预留能力
- 是否引入新的 I/O 开销、锁竞争或缓存边界问题
- 前后端字段命名与行为是否一致（尤其是 LogEntry 字段）
- 新增异步代码是否有取消支持（`CancellationToken`）
- 文档是否仍然准确
- 测试是否覆盖了边界条件（空查询、时间过滤格式异常等）

---

## CI 流水线说明

`.github/workflows/ci.yml` 触发条件：Push 到 `main`/`develop`，或 PR。

| Job | 平台 | 检查内容 |
|-----|------|---------|
| `test-rust` | Linux + Windows + macOS + Apple Silicon | cargo fmt, clippy, cargo test |
| `test-frontend` | Ubuntu | eslint, tsc, jest, vite build |
| `desktop-smoke` | Ubuntu | 完整 Tauri debug 构建 |
| `integration-test` | Ubuntu | `cargo test --all-features`（workspace 全量） |

本地复现 CI 检查：

```bash
cd log-analyzer
npm run validate:ci
```

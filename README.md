# Log Analyzer

面向开发与运维场景的本地桌面日志分析工具，技术栈为 `Rust + Tauri 2 + React 19 + TypeScript`。

## 项目现状

当前仓库的真实主链路是：

- 工作区导入到 CAS 与 SQLite 元数据存储
- 搜索由 `search_logs` / `fetch_search_page` 驱动
- 关键词匹配由 `QueryExecutor + QueryPlanner + RegexEngine` 执行
- 结果通过磁盘分页返回前端，避免一次性加载大结果集

说明：

- README 和 `docs/` 已按当前代码结构收敛，只保留长期维护的核心文档。
- 一次性分析报告、修复计划、AI 工具说明和过时迁移文档已移除。

## 主要能力

- 本地日志搜索，支持简单多关键词查询，关键词之间使用 `|` 表示 OR 逻辑
- 基于 `Aho-Corasick` 与 `regex` 的逐行匹配
- 搜索结果磁盘直写分页，适配大结果集
- 按日志级别、时间范围、文件模式过滤
- ZIP / TAR / GZ / RAR / 7Z 等压缩格式导入
- CAS 去重存储与 SQLite 元数据管理
- 虚拟文件树、导出、文件监听、性能监控

## 仓库结构

```text
log-analyzer_rust/
├── log-analyzer/                 # 前端与 Tauri 应用
│   ├── src/                      # React 前端
│   └── src-tauri/                # Rust 后端主 crate
│       └── crates/               # la-core / la-storage / la-search / la-archive
├── docs/                         # 核心文档
└── scripts/                      # CI / 校验脚本
```

## 快速开始

环境要求：

- Node.js `>= 22.12.0`
- npm `>= 10`
- Rust `>= 1.70`
- 对应平台的 Tauri 前置依赖

开发运行：

```bash
git clone https://github.com/ashllll/log-analyzer_rust.git
cd log-analyzer_rust/log-analyzer
npm install
npm run tauri dev
```

生产构建：

```bash
cd log-analyzer
npm run tauri build
```

## 常用检查

前端：

```bash
cd log-analyzer
npm run lint
npm run type-check
npm test
```

Rust：

```bash
cd log-analyzer/src-tauri
cargo fmt -- --check
cargo clippy --all-features --all-targets -- -D warnings
cargo test -q
```

## 核心文档

- [文档索引](./docs/README.md)
- [贡献指南](./docs/CONTRIB.md)
- [运行手册](./docs/RUNBOOK.md)
- [发布流程](./RELEASE_PROCESS.md)
- [IPC API 概览](./docs/architecture/API.md)
- [CAS 架构](./docs/architecture/CAS_ARCHITECTURE.md)
- [模块架构](./docs/architecture/modules/MODULE_ARCHITECTURE.md)
- [搜索优化与边界条件审核](./docs/search-optimization-review.md)

## 开发说明

- 前端真实搜索入口在 [SearchPage.tsx](/Users/llll/code/github/log-analyzer_rust/log-analyzer/src/pages/SearchPage.tsx)
- 后端真实搜索入口在 [search.rs](/Users/llll/code/github/log-analyzer_rust/log-analyzer/src-tauri/src/commands/search.rs)
- 结构化查询相关代码已存在，但当前 UI 主搜索仍以简单字符串查询为主
- 文档以“当前代码真实行为”为准，不把预留能力当作已投入主链路的能力

## License

Apache-2.0

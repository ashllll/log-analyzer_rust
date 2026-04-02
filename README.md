# Log Analyzer

面向开发与运维场景的本地桌面日志分析工具，技术栈为 `Rust + Tauri 2 + React 19 + TypeScript`。

## 技术栈

| 层次 | 技术 |
|------|------|
| 桌面框架 | Tauri 2.0 |
| 后端语言 | Rust 1.70+ |
| 前端框架 | React 19 + TypeScript 5.8 |
| 状态管理 | Zustand |
| 数据获取 | TanStack Query (React Query) |
| 样式 | Tailwind CSS |
| 数据库 | SQLite (WAL 模式) |
| 异步运行时 | Tokio |
| 构建工具 | Vite |

## 主要能力

- 本地日志搜索，支持多关键词查询，关键词之间使用 `|` 表示 OR 逻辑
- 基于 `Aho-Corasick` 多模式引擎与 `regex` 的逐行匹配
- 搜索结果磁盘直写分页，适配大结果集，避免内存溢出
- 按日志级别、时间范围、文件模式过滤（支持通配符）
- ZIP / TAR / GZ / RAR / 7Z 等压缩格式导入，支持嵌套归档递归解压
- CAS（内容寻址存储）去重，SHA-256 哈希标识，避免重复内容占用磁盘
- SQLite 元数据管理，记录虚拟路径、归档层级关系
- 虚拟文件树浏览、文件内容导出
- 文件监听（inotify/FSEvents/ReadDirectoryChangesW），实时增量日志追踪
- 性能监控：搜索延迟、缓存命中率、导入吞吐量等指标

## 仓库结构

```text
log-analyzer_rust/
├── log-analyzer/                         # 前端与 Tauri 应用根目录
│   ├── src/                              # React 前端
│   │   ├── pages/                        # 页面组件
│   │   ├── components/                   # 通用 UI 组件
│   │   ├── hooks/                        # React 自定义 Hook
│   │   ├── services/                     # Tauri IPC 封装
│   │   ├── stores/                       # Zustand 状态仓库
│   │   ├── types/                        # TypeScript 类型定义
│   │   └── utils/                        # 工具函数
│   ├── src-tauri/                        # Rust 后端主 crate
│   │   ├── src/
│   │   │   ├── commands/                 # Tauri IPC 命令层（40+ 命令）
│   │   │   ├── services/                 # 业务逻辑层（查询、匹配、监听）
│   │   │   ├── storage/                  # 存储适配层
│   │   │   ├── search_engine/            # 搜索引擎基础设施
│   │   │   ├── archive/                  # 归档处理适配层
│   │   │   ├── monitoring/               # 性能指标采集
│   │   │   ├── task_manager/             # 后台任务调度
│   │   │   ├── state_sync/               # 前后端状态同步
│   │   │   ├── models/                   # 后端内部模型
│   │   │   └── utils/                    # 工具函数
│   │   ├── crates/                       # Workspace 子 crate
│   │   │   ├── la-core/                  # 公共类型、错误、Trait
│   │   │   ├── la-storage/               # CAS 与元数据存储实现
│   │   │   ├── la-search/                # 搜索结果存储与高级索引基础设施
│   │   │   └── la-archive/               # 归档格式处理核心逻辑
│   │   ├── config/                       # 配置模板
│   │   └── Cargo.toml                    # Workspace 清单（版本 1.2.56）
│   ├── package.json
│   └── vite.config.ts
├── docs/                                 # 核心文档
│   ├── architecture/                     # 架构文档
│   └── search-optimization-review.md    # 搜索优化审核记录
├── scripts/                              # CI / 校验脚本
├── .github/workflows/                    # CI/CD 流水线
└── test_nested_archives/                 # 测试用嵌套归档夹具
```

## 核心搜索链路

当前主搜索链路（非 Tantivy 全文索引路径）：

```text
SearchPage.tsx
→ api.searchLogs(query, filters)
→ commands/search.rs: search_logs
  → QueryValidator 校验查询合法性
  → MetadataStore::get_all_files() 获取候选文件列表
  → 文件级 filePattern 过滤（早筛，支持通配符）
  → 逐文件：CAS::retrieve() 读取内容
  → QueryExecutor / RegexEngine 逐行匹配
    → OR 多关键词：Aho-Corasick 多模式快速预检
    → 命中后提取 MatchDetail 详情
  → 行级时间/级别过滤（分段摘要早筛）
  → DiskResultStore 写盘（bincode 序列化，分页）
→ 返回 { search_id, total_count }

前端分页拉取：
→ fetch_search_page(search_id, offset, limit)
→ DiskResultStore 按偏移读取
→ 渲染分页结果 + 关键词高亮
```

## 快速开始

### 环境要求

| 工具 | 版本要求 |
|------|---------|
| Node.js | >= 22.12.0 |
| npm | >= 10 |
| Rust | >= 1.70 |
| Tauri 前置依赖 | 见平台说明 |

平台前置依赖参考 [Tauri 2 官方文档](https://v2.tauri.app/start/prerequisites/)。

### 开发运行

```bash
git clone https://github.com/ashllll/log-analyzer_rust.git
cd log-analyzer_rust/log-analyzer
npm install
npm run tauri dev
```

### 生产构建

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

| 文档 | 说明 |
|------|------|
| [文档索引](./docs/README.md) | 文档目录与阅读顺序 |
| [贡献指南](./docs/CONTRIB.md) | 开发环境、提交流程、测试约定 |
| [运行手册](./docs/RUNBOOK.md) | 构建、排障、回滚指南 |
| [发布流程](./RELEASE_PROCESS.md) | 版本发布步骤与校验要求 |
| [IPC API 概览](./docs/architecture/API.md) | 40+ Tauri 命令与事件接口说明 |
| [CAS 存储架构](./docs/architecture/CAS_ARCHITECTURE.md) | 内容寻址存储与元数据设计 |
| [模块架构](./docs/architecture/modules/MODULE_ARCHITECTURE.md) | 各模块职责边界与调用关系详解 |
| [搜索优化审核](./docs/search-optimization-review.md) | 搜索性能边界条件与优化记录 |

## 开发说明

- 前端搜索入口：`log-analyzer/src/pages/SearchPage.tsx`
- 后端搜索入口：`log-analyzer/src-tauri/src/commands/search.rs`
- 当前 UI 主搜索使用简单字符串查询，`|` 分隔表示 OR
- 结构化查询（`execute_structured_query`）能力已存在，但不是当前 UI 主搜索入口
- 文档以"当前代码真实行为"为准，不将预留能力误写为已投入主链路

## License

Apache-2.0

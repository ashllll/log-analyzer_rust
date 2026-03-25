# Log Analyzer

<div align="center">

![Version](https://img.shields.io/badge/version-1.2.53-brightgreen.svg)
![License](https://img.shields.io/badge/license-Apache%202.0-blue.svg)
![Rust](https://img.shields.io/badge/Rust-1.70+-orange.svg)
![Tauri](https://img.shields.io/badge/Tauri-2.0-FFC131.svg?logo=tauri)
![React](https://img.shields.io/badge/React-19.1.0-61dafb.svg?logo=react)
![TypeScript](https://img.shields.io/badge/TypeScript-5.8.3-3178c6.svg?logo=typescript)

**面向开发者与运维人员的高性能桌面日志分析工具**

[快速开始](#快速开始) · [功能特性](#功能特性) · [架构设计](#架构设计) · [开发指南](#开发指南) · [文档](#文档)

</div>

---

## 简介

Log Analyzer 是一款采用 **Rust + Tauri 2.0 + React 19** 技术栈打造的跨平台桌面日志分析工具。

- **高性能搜索**：Aho-Corasick 多模式匹配 + Tantivy 全文搜索引擎，搜索响应 <200ms
- **智能存储**：CAS（内容寻址存储）+ SQLite FTS5，自动去重，节省磁盘空间 30%+
- **全格式支持**：ZIP / TAR / GZ / RAR / 7Z，支持 7 层嵌套压缩包递归解析
- **完全离线**：所有数据本地处理，零网络传输，隐私安全有保障
- **插件系统**：基于 `libloading` 的动态库加载，ABI 版本校验 + 目录白名单防护
- **可靠性保障**：Saga 事务模式、垃圾回收、缓存一致性监控，确保数据完整性

---

## 功能特性

### 搜索引擎

| 特性 | 说明 |
|------|------|
| Aho-Corasick 算法 | 多模式匹配 O(n+m)，性能提升 80%+ |
| Tantivy 全文搜索 | 布尔查询、短语查询、模糊查询，<200ms 响应 |
| 查询成本估算 | 智能成本模型，优化布尔查询执行顺序 |
| 细粒度取消机制 | 每 256 个文档检查一次，支持搜索中断 |
| 关键词高亮 | 多关键词自动着色，大文档字节级切片优化 |
| 正则表达式 | 完整 RE2 语法支持，LRU 编译缓存 |
| 流式分页 | 超过 5,000 条结果自动切换流式加载，避免内存压力 |
| 智能截断 | 超过 1,000 字符的行自动截断，可展开查看全文 |
| 时间戳分区 | 智能处理无效时间戳，避免污染时间线 |

### 文件与压缩包

- **支持格式**：`.log` `.txt` 及所有文本格式；`.zip` `.tar` `.tar.gz` `.tgz` `.gz` `.rar` `.7z`
- **递归解压**：自动处理嵌套压缩包，最深支持 7 层
- **流式解压**：大文件增量读取，内存占用可控（GZ 超过 1MB 触发流式）
- **路径安全**：防止路径遍历攻击，符号链接深度防御（O_NOFOLLOW）
- **容量限制**：ZIP 单文件 500MB 硬限制，防止内存炸弹

### 存储系统

- **CAS 架构**：SHA-256 内容寻址，文件内容与路径解耦，相同内容只存一份
- **SQLite + FTS5**：全文搜索索引，查询性能提升 10 倍+
- **并发安全**：`DashMap` 无锁并发哈希映射，`parking_lot` 高性能锁
- **数据完整性**：SHA-256 哈希校验，防止静默数据损坏
- **批量插入优化**：RETURNING 子句支持，99%+ 性能提升
- **Saga 事务模式**：跨存储原子操作，自动回滚与孤儿文件清理
- **垃圾回收**：自动清理无引用 CAS 对象，可配置策略与后台运行
- **缓存一致性**：双检查模式防止 TOCTOU 竞态，实时监控与自动修复

### 用户界面

- **虚拟滚动**：`@tanstack/react-virtual`，轻松渲染百万级日志记录
- **关键词组**：多组关键词按颜色分类（蓝/绿/橙/红/紫），支持正则表达式
- **智能过滤**：文件过滤规则，支持包含/排除模式
- **实时监听**：自动监听工作区文件变化，增量索引新内容
- **CSV 导出**：搜索结果导出为 UTF-8 BOM CSV 格式
- **国际化**：中英文完整支持（i18next）
- **性能仪表盘**：实时展示 CPU、内存、搜索吞吐量等指标

### 任务系统

- **Actor 模型**：异步任务管理，前端实时展示导入进度
- **熔断自愈**：Circuit Breaker 模式，出现异常自动恢复
- **后台任务**：导入、索引、监听均在后台异步执行，不阻塞 UI

---

## 下载安装

从 [Releases](https://github.com/ashllll/log-analyzer_rust/releases/latest) 下载对应平台安装包：

| 平台 | 文件 |
|------|------|
| Windows | `log-analyzer_*_x64-setup.exe` |
| macOS (Intel) | `log-analyzer_*_x64.dmg` |
| macOS (Apple Silicon) | `log-analyzer_*_aarch64.dmg` |
| Linux (便携版) | `log-analyzer_*_amd64.AppImage` |
| Linux (Debian/Ubuntu) | `log-analyzer_*_amd64.deb` |

---

## 快速开始

### 环境要求

| 工具 | 版本 |
|------|------|
| Node.js | 22.12.0+ |
| npm | 10.0+ |
| Rust | 1.70+ |

Linux 额外需要 GTK3/GTK4 开发库，详见 [Tauri 前置依赖](https://tauri.app/v1/guides/getting-started/prerequisites)。

### 安装与运行

```bash
# 克隆仓库
git clone https://github.com/ashllll/log-analyzer_rust.git
cd log-analyzer_rust/log-analyzer

# 安装前端依赖
npm install

# 启动开发服务器（Tauri + Vite HMR）
npm run tauri dev

# 构建生产版本
npm run tauri build
```

### 质量检查

```bash
# 前端
npm run type-check     # TypeScript 类型检查
npm run lint           # ESLint 检查
npm test               # Jest 单元测试

# Rust 后端
cd src-tauri
cargo fmt -- --check   # 格式检查
cargo clippy --all-features --all-targets -- -D warnings  # Clippy 检查
cargo test --all-features --lib --bins  # 单元测试

# 完整 CI 验证（推送前）
npm run validate:ci
```

---

## 架构设计

### 目录结构

```
log-analyzer_rust/
├── log-analyzer/
│   ├── src/                        # React 前端
│   │   ├── components/             # UI 组件
│   │   │   ├── modals/             # 弹窗（关键词、文件过滤等）
│   │   │   ├── renderers/         # 日志渲染器（虚拟滚动、高亮）
│   │   │   ├── search/            # 搜索相关组件
│   │   │   └── ui/                # 基础 UI 组件
│   │   ├── pages/                  # 页面（Search / Workspaces / Keywords / Tasks / Performance / Settings）
│   │   ├── services/               # API 封装、SearchQueryBuilder
│   │   ├── hooks/                  # 自定义 Hooks
│   │   ├── stores/                 # Zustand 全局状态
│   │   ├── types/                  # TypeScript 类型定义
│   │   ├── constants/               # 颜色常量、搜索配置
│   │   └── i18n/                   # 国际化（zh.json / en.json）
│   └── src-tauri/                  # Rust 后端
│       └── src/
│           ├── application/         # 应用接入层（插件系统、命令注册）
│           ├── commands/            # Tauri 命令（search / import / workspace / watch 等）
│           ├── search_engine/       # 搜索引擎（Tantivy、布尔查询、高亮、流式构建器）
│           ├── services/             # 业务逻辑（PatternMatcher、QueryExecutor 等）
│           ├── storage/            # CAS 存储 + SQLite 元数据
│           ├── archive/            # 压缩包处理（ZIP/RAR/GZ/TAR/7Z，熔断自愈）
│           ├── task_manager/         # 异步任务 Actor 模型
│           ├── monitoring/          # 可观测性（metrics、OpenTelemetry）
│           ├── domain/              # 领域模型
│           └── models/              # 数据模型
├── docs/                           # 项目文档
│   └── 技术文档.md                  # 综合技术文档
└── scripts/                        # CI 验证脚本
```

### CAS 存储架构

```
workspace/
├── objects/            # 内容寻址对象（SHA-256 前两位为目录）
│   ├── ab/
│   │   └── cdef1234...  # 完整哈希为文件名
│   └── ...
└── metadata.db         # SQLite 数据库
    ├── files            # 文件元数据（哈希、虚拟路径、大小等）
    ├── archives         # 压缩包嵌套关系
    ├── files_fts        # FTS5 全文搜索索引
    └── index_state      # 增量索引状态
```

**核心组件**

| 组件 | 文件 | 职责 |
|------|------|------|
| ContentAddressableStorage | `storage/cas.rs` | SHA-256 内容寻址、对象存储、缓存管理 |
| MetadataStore | `storage/metadata_store.rs` | SQLite 元数据管理、FTS5 全文搜索 |
| StorageCoordinator | `storage/coordinator.rs` | Saga 事务协调、原子性保证 |
| GarbageCollector | `storage/gc.rs` | 孤儿文件清理、引用计数、后台 GC |
| CacheMonitor | `storage/cache_monitor.rs` | 缓存一致性监控、健康指标 |

**数据流**

```
导入：原始文件 → SHA-256 → 去重检查 → objects/ → Saga事务 → SQLite 元数据
      ↓
      失败时：自动清理孤儿文件（引用计数为零）

搜索：用户查询 → Tantivy / FTS5 → 哈希列表 → CAS 读取内容 → 结果返回
      ↓
      缓存未命中时：双检查模式防止 TOCTOU 竞态
```

**可靠性保障**

- **Saga 事务模式**：CAS 写入与元数据插入的原子操作，失败自动补偿
- **TempFileGuard**：RAII 模式确保临时文件清理（即使 panic 也能保证）
- **双检查缓存**：缓存命中后验证文件系统状态，自动修复不一致
- **TOCTOU 防护**：`O_EXCL` 标志确保原子文件创建，防止竞态条件
- **后台垃圾回收**：自动清理无引用 CAS 对象，可配置策略（间隔、年龄阈值）
- **缓存监控**：实时跟踪命中率、检测并修复陈旧缓存条目

### 前后端通信规范

Tauri IPC 严格使用 `snake_case`，前后端字段名必须完全一致：

```typescript
// TypeScript 前端
interface TaskInfo {
  task_id: string;      // 与 Rust 完全一致
  task_type: string;
}
```

```rust
// Rust 后端
#[derive(Serialize, Deserialize)]
pub struct TaskInfo {
    pub task_id: String,
    pub task_type: String,
}
```

---

## 技术栈

### 前端

| 技术 | 版本 | 用途 |
|------|------|------|
| React | 19.1.0 | UI 框架 |
| TypeScript | 5.8.3 | 类型安全 |
| Vite | 7.x | 构建工具 |
| Zustand | 5.0.9 | 全局状态管理 |
| TanStack Query | 5.90.12 | 服务端状态 |
| TanStack Virtual | 3.13.12 | 虚拟滚动 |
| React Router | 7.0.0 | 路由导航 |
| Tailwind CSS | 3.4.17 | 样式系统 |
| Framer Motion | 12.23.24 | 动画 |
| Zod | 4.3.6 | 运行时数据验证 |
| i18next | 25.7.1 | 国际化 |
| Recharts | 3.x | 性能图表 |

### 后端

| 技术 | 版本 | 用途 |
|------|------|------|
| Tauri | 2.0 | 桌面应用框架 |
| tokio | 1.x | 异步运行时 |
| Tantivy | 0.22 | 全文搜索引擎 |
| aho-corasick | 1.1 | 多模式字符串匹配 |
| sqlx | 0.7 | SQLite 异步访问（FTS5） |
| parking_lot | 0.12 | 高性能锁 |
| dashmap | 5.5 | 并发哈希映射 |
| rayon | 1.8 | 并行搜索 |
| moka | 0.12 | 企业级异步缓存 |
| tracing | 0.1 | 结构化日志与追踪 |
| thiserror | 1.0 | 错误类型定义 |
| eyre / miette | 0.6 / 5.0 | 错误报告 |
| libloading | 0.8 | 插件动态库加载 |
| notify | 6.1 | 文件系统监听 |
| zip / tar / flate2 | 0.6 / 0.4 / 1.0 | 压缩包处理 |
| async_zip | 0.0.17 | 异步 ZIP 处理 |
| sevenz-rust | 0.5 | 7Z 格式支持 |
| unrar | 0.5 | RAR 格式支持（libunrar 绑定） |
| sha2 | 0.10 | SHA-256 内容寻址 |
| governor | 0.6 | 速率限制 |
| dunce | 1.0 | Windows UNC 路径规范化 |

### 测试

| 工具 | 用途 |
|------|------|
| rstest | Rust 参数化单元测试 |
| proptest | 属性测试（模糊测试） |
| criterion | 性能基准测试 |
| Jest + RTL | 前端组件测试 |
| fast-check | 前端属性测试 |

---

## 性能基准

| 指标 | 数值 |
|------|------|
| 搜索吞吐量 | 10,000+ 次/秒 |
| 单关键词搜索 | <10ms |
| 多关键词搜索（10 个） | <50ms |
| Tantivy 全文查询 | <200ms |
| ZIP 解压（100MB） | <5 秒 |
| 索引构建（10,000 行） | <1 秒 |
| 增量更新（1,000 行） | <100ms |
| 空闲内存占用 | <100MB |
| 加载 1GB 日志内存 | <500MB |
| CAS 去重空间节省 | 30%+ |
| 批量插入优化 | 99%+ 提升（RETURNING 子句） |

---

## 应用数据目录

| 平台 | 路径 |
|------|------|
| Windows | `%APPDATA%\com.joeash.log-analyzer\workspaces\` |
| macOS | `~/Library/Application Support/com.joeash.log-analyzer/workspaces/` |
| Linux | `~/.local/share/com.joeash.log-analyzer/workspaces/` |

---

## 开发指南

### 添加 Tauri 命令

1. 在 `src-tauri/src/commands/` 创建新文件
2. 用 `#[tauri::command]` 装饰函数
3. 在 `commands/mod.rs` 导出
4. 在 `lib.rs` 的 `invoke_handler()` 注册
5. 前端用 `invoke()` 调用，参数键名用 `snake_case`

### 添加前端页面

1. 在 `src/pages/` 创建组件
2. 在 `src/i18n/zh.json` 和 `en.json` 添加文案
3. 在导航中添加路由

### Git Pre-Push 验证

项目使用 Husky v9 配置 pre-push hook，推送前自动执行完整 CI 检查：

```bash
# 手动触发
npm run validate:ci
```

检查项：ESLint、TypeScript 类型、前端测试、前端构建、Rust 格式、Clippy、Rust 测试。

---

## 常见问题

<details>
<summary>支持哪些日志格式？</summary>

支持所有文本格式（`.log`、`.txt` 等）及压缩格式（`.zip`、`.tar`、`.tar.gz`、`.gz`、`.rar`、`.7z`）。
</details>

<details>
<summary>是否需要网络连接？</summary>

不需要。应用完全离线运行，所有数据处理在本地完成。
</details>

<details>
<summary>支持多少层嵌套压缩包？</summary>

最多支持 7 层嵌套（例如：`.zip` → `.tar` → `.gz` → `.zip` → `.tar` → `.gz` → `.log`）。
</details>

<details>
<summary>Linux 下 GTK 依赖问题？</summary>

```bash
# 检查已安装版本
pkg-config --modversion gtk+-3.0
pkg-config --modversion gtk4

# 按返回版本安装对应开发包
sudo apt install libgtk-3-dev   # GTK3
sudo apt install libgtk-4-dev   # GTK4
```
</details>

<details>
<summary>Windows 路径过长报错？</summary>

应用已使用 `dunce` crate 处理 Windows UNC 路径，CAS 架构本身也规避了路径长度限制。
</details>

<details>
<summary>如何处理大文件导入性能问题？</summary>

代码使用 RETURNING 子句优化批量插入，减少数据库往返次数 99%+，建议更新到最新版本以获得最佳性能。
</details>

---

## 文档

| 文档 | 说明 |
|------|------|
| [CLAUDE.md](CLAUDE.md) | 项目开发指南（架构决策、编码规范、缺陷分析） |
| [CHANGELOG.md](CHANGELOG.md) | 版本更新历史 |
| [docs/技术文档.md](docs/技术文档.md) | 综合技术文档 |
| [docs/architecture/CAS_ARCHITECTURE.md](docs/architecture/CAS_ARCHITECTURE.md) | CAS 存储架构详解 |
| [docs/architecture/API.md](docs/architecture/API.md) | Tauri 命令 API 文档 |
| [docs/guides/QUICK_REFERENCE.md](docs/guides/QUICK_REFERENCE.md) | 用户快速参考手册 |
| [docs/guides/MULTI_KEYWORD_SEARCH_GUIDE.md](docs/guides/MULTI_KEYWORD_SEARCH_GUIDE.md) | 多关键词搜索指南 |

---

## 贡献

1. Fork 本仓库
2. 创建特性分支：`git checkout -b feature/my-feature`
3. 提交更改：`git commit -m 'feat(scope): 描述'`
4. 推送分支：`git push origin feature/my-feature`
5. 开启 Pull Request

**要求**：所有代码须通过 `npm run validate:ci`，新功能需添加对应测试。

---

## 许可证

本项目采用 **Apache License 2.0**，详见 [LICENSE](LICENSE) 文件。

---

<div align="center">

由 [ashllll](https://github.com/ashllll) 用 Rust + React 打造

[报告问题](https://github.com/ashllll/log-analyzer_rust/issues) · [功能建议](https://github.com/ashllll/log-analyzer_rust/issues)

</div>

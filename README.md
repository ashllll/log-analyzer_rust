# Log Analyzer

<div align="center">

**🚀 高性能桌面日志分析工具**

基于 Rust + Tauri + React / Flutter 构建的现代化日志分析平台

[![Rust](https://img.shields.io/badge/Rust-1.70+-orange.svg)](https://www.rust-lang.org/)
[![Tauri](https://img.shields.io/badge/Tauri-2.0-blue.svg)](https://tauri.app/)
[![React](https://img.shields.io/badge/React-19.1.0-61dafb.svg)](https://reactjs.org/)
[![Flutter](https://img.shields.io/badge/Flutter-3.8+-02569B.svg)](https://flutter.dev/)
[![TypeScript](https://img.shields.io/badge/TypeScript-5.8.3-3178c6.svg)](https://www.typescriptlang.org/)
[![License](https://img.shields.io/badge/license-Apache%202.0-green.svg)](LICENSE)
[![Version](https://img.shields.io/badge/version-0.0.143-brightgreen.svg)](CHANGELOG.md)

[快速开始](#-快速开始) · [核心特性](#-核心特性) · [技术架构](#-技术架构) · [开发指南](#-开发指南) · [文档](#-文档)

</div>

---

## 📖 项目简介

Log Analyzer 是一款面向开发者和运维人员的**专业级桌面日志分析工具**，采用 Rust + Tauri + React / Flutter 现代技术栈打造，提供极致的性能体验和可靠的数据处理能力。

### 🎯 设计理念

- **性能至上**: Aho-Corasick算法 + Tantivy搜索引擎，搜索响应 <200ms
- **数据安全**: Git风格CAS存储系统，原子操作防止数据损坏
- **隐私优先**: 所有数据本地处理，零网络传输，完全离线可用
- **开发体验**: 清晰的架构设计，99.8%测试覆盖率，零clippy警告
- **多前端支持**: React (Tauri) + Flutter (跨平台) 双前端架构

### 🏆 核心优势

| 维度 | 指标 | 说明 |
|------|------|------|
| **性能** | 10,000+ 搜索/秒 | Aho-Corasick 多模式匹配，O(n+m) 复杂度 |
| **存储** | 节省空间 30%+ | 内容寻址存储 (CAS)，SHA-256 去重 |
| **搜索** | <200ms 响应 | Tantivy 全文引擎 + Aho-Corasick 多模式匹配 |
| **稳定** | 熔断自愈 | Circuit Breaker 捕获 Panic 传播，支持锁中毒自动恢复 |
| **安全** | 深度防御 | 插件白名单 + ABI 验证 + 路径递归扫描 |
| **测试** | 534/535 通过 | 99.8% 覆盖率，集成属性测试 (Proptest) |
| **验证** | Zod 类型安全 | 运行时类型验证 + 编译时类型检查 |
| **扩展** | HTTP API + FFI | 支持 Flutter/第三方调用 |

---

## ✨ 核心特性

### 🔍 智能搜索系统

- **Aho-Corasick算法**: 多模式匹配，搜索性能提升80%+
- **Tantivy搜索引擎**: 子200ms响应，支持高级查询语法
- **并发搜索**: 流式处理，支持大规模批量查询
- **查询优化器**: 自动识别慢查询，提供优化建议
- **布尔逻辑**: AND/OR/NOT组合查询，精准定位
- **正则表达式**: 完整的正则支持，复杂模式匹配
- **实时高亮**: 搜索结果自动高亮，支持多关键词
- **查询构建器**: SearchQueryBuilder 流畅 API 设计
- **模糊搜索**: Levenshtein 距离算法，容忍拼写错误
- **搜索历史**: 保存最近 50 条记录，智能去重

### 📦 多格式支持

- **压缩格式**: ZIP, TAR, GZ, RAR, 7Z 等主流格式
- **递归解压**: 自动处理任意层级嵌套（zip→tar.gz→log）
- **路径安全**: 统一路径验证器，防止路径遍历攻击
- **流式处理**: 大文件增量读取，内存占用可控
- **RAR优化**: `unrar` crate 实现，无 sidecar 依赖
- **七层解压**: 支持 ZIP→TAR→GZ→ZIP→TAR→GZ→LOG 七层嵌套

### 💾 CAS存储系统

- **Git风格设计**: SHA-256内容寻址，文件内容与路径解耦
- **自动去重**: 相同内容只存储一次，节省磁盘空间30%+
- **原子操作**: O_EXCL标志，消除TOCTOU竞态条件
- **SQLite索引**: FTS5全文搜索，查询性能提升10倍
- **并发安全**: DashSet缓存，支持高并发读写
- **UNIQUE约束**: INSERT OR IGNORE + SELECT 模式处理幂等性

### 🎨 现代化UI

- **虚拟滚动**: 轻松处理百万级日志记录
- **智能截断**: 长日志自动截断，保留关键词上下文
- **关键词统计**: 实时显示各关键词匹配数量和占比
- **国际化**: 完整的中英文支持（i18next / Flutter intl）
- **响应式设计**: Tailwind CSS，适配各种屏幕尺寸
- **暗色模式**: 护眼配色，长时间使用更舒适
- **性能监控**: 实时系统性能指标展示

### 🔐 安全加固

- **路径验证**: 深度递归验证算法，防止路径遍历、符号链接及归档炸弹攻击
- **原子写入**: CAS存储系统采用 O_EXCL 原子标志，彻底消除 TOCTOU 竞态条件
- **插件安全**: 动态库加载目录白名单验证 + ABI 版本匹配检查，防止恶意代码注入
- **故障恢复**: 熔断器机制集成锁中毒（Poisoning）自动恢复，确保系统在并发错误下的高可用
- **错误处理**: 生产代码 100% 消除 `unwrap/expect`，采用 `eyre` 结构化错误链处理
- **内存限制**: 针对不同归档格式实现流式配额限制，防止内存耗尽攻击 (OOM)
- **表单验证**: Zod v4.3.6 运行时类型验证 + 编译时类型推导

### ⚡ 性能优化

- **流式并发**: 内存使用从O(n)降至O(max_concurrent)
- **GZ优化**: 内存峰值降低89%（99MB→10MB）
- **查询分析**: 自动识别热查询，优化索引策略
- **异步I/O**: tokio非阻塞操作，UI响应性极佳
- **React Query**: 智能缓存和数据同步，减少重复请求
- **React Virtual**: 高性能虚拟滚动，O(1) 渲染复杂度

### 🛡️ 错误处理系统

- **多层错误边界**: React Error Boundary 全局捕获
- **结构化错误**: ErrorCode 枚举 + ApiError 类
- **错误日志持久化**: localStorage 存储，7天保留，最多100条
- **Toast 通知**: 生产环境友好的错误提示
- **错误去重**: 5秒防抖机制，避免重复提示

### 🔌 双前端架构

- **React + Tauri**: 桌面应用首选，原生性能
- **Flutter + Rust FFI**: 跨平台移动端支持
- **HTTP API**: 统一后端服务，支持第三方集成
- **FFI 桥接**: flutter_rust_bridge 2.x 高性能通信

---

## 🚀 快速开始

### 环境要求

| 工具 | 版本要求 | 说明 |
|------|---------|------|
| Node.js | 22.12.0+ | JavaScript运行时 |
| npm | 10.0+ | 包管理器 |
| Rust | 1.70+ | 系统编程语言 |
| Cargo | 随Rust安装 | Rust包管理器 |
| Flutter | 3.8+ | 移动端开发（可选） |

**系统依赖**: 参考 [Tauri Prerequisites](https://tauri.app/v1/guides/getting-started/prerequisites)

### 安装步骤

#### React + Tauri 版本

```bash
# 1. 克隆仓库
git clone https://github.com/ashllll/log-analyzer_rust.git
cd log-analyzer_rust

# 2. 安装依赖（在 log-analyzer 目录下）
cd log-analyzer
npm install

# 3. 启动开发服务器
npm run tauri dev

# 4. 构建生产版本（可选）
npm run tauri build
```

#### Flutter 版本（实验性）

```bash
# 1. 进入 Flutter 项目目录
cd log-analyzer_rust/log-analyzer_flutter

# 2. 安装依赖
flutter pub get

# 3. 生成 FFI 桥接代码
flutter_rust_bridge_codegen generate

# 4. 运行应用
flutter run
```

### 快速验证

```bash
# 运行 Rust 后端测试
cd log-analyzer/src-tauri
cargo test --all-features

# 代码质量检查
cargo clippy --all-features -- -D warnings

# 前端检查（如果有前端代码）
cd ..
npm run type-check
npm run lint
```

---

## 📚 使用指南

### 基础工作流

#### 1️⃣ 创建工作区

1. 启动应用，点击左侧 **"Workspaces"** 标签
2. 点击 **"Import File"** 或 **"Import Folder"**
   - Import File: 导入单个日志文件或压缩包
   - Import Folder: 递归导入整个文件夹
3. 等待处理完成（查看"Background Tasks"标签）

**支持格式**: `.log`, `.txt`, `.zip`, `.tar`, `.gz`, `.rar`, `.7z` 等

#### 2️⃣ 搜索日志

1. 点击 **"Search"** 标签
2. 输入搜索关键词
   - 单关键词: `error`
   - 多关键词: `error|warning|critical`
   - 正则表达式: `ERROR.*timeout`
   - 布尔查询: `error AND (timeout OR critical)`
3. 按 **Enter** 执行搜索
4. 查看结果和统计面板

**搜索技巧**:
- 关键词统计显示每个词的匹配数
- 长日志智能截断，保留上下文
- 所有匹配词高亮显示
- 支持虚拟滚动快速浏览
- 模糊搜索容忍拼写错误

#### 3️⃣ 配置关键词组

1. 点击 **"Keywords"** 标签
2. 创建关键词组，设置颜色和模式
3. 在搜索页面快速应用过滤器
4. 支持导入/导出配置

#### 4️⃣ 监控性能

1. 点击 **"Performance"** 标签
2. 查看实时性能指标
3. 搜索延迟、吞吐量、缓存命中率
4. 系统资源使用情况

### 快捷键

| 快捷键 | 功能 |
|--------|------|
| `Cmd+K` / `Ctrl+K` | 聚焦搜索框 |
| `Enter` | 执行搜索 |
| `Esc` | 关闭详情面板 |

---

## 🏗️ 技术架构

### 技术栈

#### 前端

```json
{
  "核心框架": {
    "react": "19.1.0",
    "react-dom": "19.1.0",
    "typescript": "5.8.3",
    "vite": "7.0.4"
  },
  "UI与样式": {
    "tailwindcss": "3.4.17",
    "lucide-react": "0.554.0",
    "framer-motion": "12.23.24",
    "@tanstack/react-virtual": "3.13.12"
  },
  "状态管理": {
    "zustand": "5.0.9",
    "@tanstack/react-query": "5.90.12"
  },
  "表单与验证": {
    "zod": "4.3.6",
    "react-hook-form": "支持"
  },
  "国际化": {
    "i18next": "25.7.1",
    "react-i18next": "16.4.0"
  },
  "错误处理": {
    "react-error-boundary": "6.0.0",
    "react-hot-toast": "2.6.0"
  }
}
```

#### Flutter 前端

```yaml
核心框架:
  flutter: "3.8+"
  flutter_localizations: SDK
  
状态管理:
  flutter_riverpod: "3.0.0"
  riverpod_annotation: "3.0.0"
  
路由:
  go_router: "14.0.0"
  
Rust FFI:
  flutter_rust_bridge: "2.0.0"
  
图表:
  fl_chart: "0.70.0"
  
错误追踪:
  sentry_flutter: "8.0.0"
```

#### 后端

```toml
[核心框架]
tauri = "2.0"
tokio = { version = "1", features = ["full"] }

[FFI 桥接]
flutter_rust_bridge = "=2.11.1"

[HTTP API]
axum = "0.7"
tower = "0.4"
tower-http = { version = "0.5", features = ["cors"] }

[搜索引擎]
tantivy = "0.22"           # 全文搜索引擎
aho-corasick = "1.1"       # 多模式匹配
regex = "1.11"             # 正则表达式
roaring = "0.10"           # 位图索引

[数据库]
sqlx = { version = "0.7", features = ["sqlite"] }

[压缩支持]
zip = "0.6"
tar = "0.4"
flate2 = "1.0"
unrar = "0.5"              # RAR (libunrar 绑定)
sevenz-rust = "0.5"        # 7Z (纯 Rust)
async-compression = "0.4"  # 流式压缩

[错误处理]
thiserror = "1.0"
eyre = "0.6"
miette = "5.0"

[并发与性能]
rayon = "1.8"              # 并行处理
parking_lot = "0.12"       # 高性能锁
crossbeam = "0.8"          # 无锁数据结构
dashmap = "5.5"            # 并发哈希映射
lru = "0.12"               # LRU 缓存
moka = { version = "0.12", features = ["future", "sync"] }  # 企业级缓存

[日志与监控]
tracing = "0.1"
tracing-subscriber = "0.3"
sentry = "0.32"
prometheus = "0.13"
metrics = "0.22"

[系统信息]
sysinfo = "0.31"           # 跨平台系统监控
```

### 架构设计

```
┌─────────────────────────────────────────────────────────────────────────┐
│                          Frontend Layer                                  │
│  ┌─────────────────────────────┐  ┌─────────────────────────────────┐  │
│  │     React + Tauri App       │  │      Flutter App (实验性)        │  │
│  │  Pages · Components · Store │  │  Features · Providers · Widgets │  │
│  └──────────────┬──────────────┘  └────────────────┬────────────────┘  │
└─────────────────┼──────────────────────────────────┼───────────────────┘
                  │ Tauri IPC                        │ FFI Bridge
                  ▼                                  ▼
┌─────────────────────────────────────────────────────────────────────────┐
│                        Rust Backend (Core)                              │
│                                                                         │
│  ┌──────────────────────────────────────────────────────────────────┐  │
│  │                    Interface Layer                                │  │
│  │  ┌────────────────┐  ┌────────────────┐  ┌────────────────────┐ │  │
│  │  │ Tauri Commands │  │   HTTP API     │  │    FFI Bridge      │ │  │
│  │  │ (17 modules)   │  │ (axum :8080)   │  │ (frb_generated)    │ │  │
│  │  └────────────────┘  └────────────────┘  └────────────────────┘ │  │
│  └──────────────────────────────────────────────────────────────────┘  │
│                                     │                                   │
│  ┌──────────────────────────────────────────────────────────────────┐  │
│  │                   Application Layer                               │  │
│  │  ┌────────────────┐  ┌────────────────┐  ┌────────────────────┐ │  │
│  │  │   Handlers     │  │   Services     │  │   Task Manager     │ │  │
│  │  │ (Command/Query)│  │ (Business Logic│  │   (Actor Model)    │ │  │
│  │  └────────────────┘  └────────────────┘  └────────────────────┘ │  │
│  └──────────────────────────────────────────────────────────────────┘  │
│                                     │                                   │
│  ┌──────────────────────────────────────────────────────────────────┐  │
│  │                     Domain Layer                                  │  │
│  │  ┌────────────────┐  ┌────────────────┐  ┌────────────────────┐ │  │
│  │  │ Search Domain  │  │ Export Domain  │  │  Log Analysis      │ │  │
│  │  │ (Query Engine) │  │ (Format/Stream)│  │  (Pattern Match)   │ │  │
│  │  └────────────────┘  └────────────────┘  └────────────────────┘ │  │
│  └──────────────────────────────────────────────────────────────────┘  │
│                                     │                                   │
│  ┌──────────────────────────────────────────────────────────────────┐  │
│  │                  Infrastructure Layer                             │  │
│  │  ┌────────────────┐  ┌────────────────┐  ┌────────────────────┐ │  │
│  │  │ CAS Storage    │  │ Archive Proc.  │  │   Monitoring       │ │  │
│  │  │ (SHA-256)      │  │ (7-Level Nest) │  │   (Metrics/Trace)  │ │  │
│  │  └────────────────┘  └────────────────┘  └────────────────────┘ │  │
│  │  ┌────────────────┐  ┌────────────────┐  ┌────────────────────┐ │  │
│  │  │ Search Engine  │  │  File Watcher  │  │  Event System      │ │  │
│  │  │ (Tantivy)      │  │ (notify)       │  │  (State Sync)      │ │  │
│  │  └────────────────┘  └────────────────┘  └────────────────────┘ │  │
│  └──────────────────────────────────────────────────────────────────┘  │
│                                                                         │
│  ┌──────────────────────────────────────────────────────────────────┐  │
│  │                    Security Layer                                 │  │
│  │  Path Validation · Plugin Whitelist · ABI Check · Rate Limiting  │  │
│  └──────────────────────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────────────────────┘
```

### 项目结构

```
log-analyzer_rust/
├── log-analyzer/                    # Tauri + React 项目
│   ├── src/                         # React 前端源码
│   │   ├── components/              # UI 组件
│   │   │   ├── ui/                  # 基础 UI 组件
│   │   │   │   ├── Button.tsx
│   │   │   │   ├── Card.tsx
│   │   │   │   ├── Input.tsx
│   │   │   │   ├── ConnectionStatus.tsx
│   │   │   │   └── ...
│   │   │   └── __tests__/           # 组件测试
│   │   ├── hooks/                   # 自定义 Hooks
│   │   │   ├── useConfig.ts
│   │   │   ├── useFormValidation.ts
│   │   │   ├── useKeywordManager.ts
│   │   │   ├── useTaskManager.ts
│   │   │   ├── useWorkspaceMutations.ts
│   │   │   └── __tests__/           # Hooks 测试
│   │   ├── pages/                   # 页面组件
│   │   │   ├── SearchPage.tsx
│   │   │   ├── KeywordsPage.tsx
│   │   │   ├── WorkspacesPage.tsx
│   │   │   └── __tests__/           # 页面测试
│   │   ├── services/                # 业务服务
│   │   │   ├── api.ts               # Tauri API 封装
│   │   │   ├── errors.ts            # 错误处理服务
│   │   │   └── __tests__/           # 服务测试
│   │   ├── stores/                  # Zustand 状态管理
│   │   ├── types/                   # TypeScript 类型
│   │   ├── utils/                   # 工具函数
│   │   └── i18n/                    # 国际化
│   │
│   ├── src-tauri/                   # Rust 后端源码
│   │   ├── src/
│   │   │   ├── application/         # 应用层
│   │   │   │   ├── commands.rs      # 命令处理
│   │   │   │   ├── handlers/        # 请求处理器
│   │   │   │   ├── plugins/         # 插件系统
│   │   │   │   ├── queries/         # 查询处理
│   │   │   │   └── services/        # 应用服务
│   │   │   │
│   │   │   ├── domain/              # 领域层
│   │   │   │   ├── export/          # 导出领域
│   │   │   │   ├── log_analysis/    # 日志分析领域
│   │   │   │   ├── search/          # 搜索领域
│   │   │   │   └── shared/          # 共享值对象
│   │   │   │
│   │   │   ├── infrastructure/      # 基础设施层
│   │   │   │   ├── config/          # 配置管理
│   │   │   │   ├── external.rs      # 外部服务
│   │   │   │   ├── messaging.rs     # 消息队列
│   │   │   │   └── persistence.rs   # 持久化
│   │   │   │
│   │   │   ├── commands/            # Tauri 命令 (17个模块)
│   │   │   │   ├── search.rs        # 搜索命令
│   │   │   │   ├── async_search.rs  # 异步搜索
│   │   │   │   ├── import.rs        # 导入命令
│   │   │   │   ├── workspace.rs     # 工作区命令
│   │   │   │   ├── watch.rs         # 文件监听
│   │   │   │   ├── export.rs        # 导出命令
│   │   │   │   ├── performance.rs   # 性能监控
│   │   │   │   ├── http_api.rs      # HTTP API 服务器
│   │   │   │   ├── search_history.rs# 搜索历史
│   │   │   │   └── ...
│   │   │   │
│   │   │   ├── ffi/                 # FFI 桥接层
│   │   │   │   ├── bridge.rs        # 主桥接实现
│   │   │   │   ├── bridge_minimal.rs# 最小桥接
│   │   │   │   ├── commands_bridge.rs
│   │   │   │   ├── global_state.rs  # 全局状态
│   │   │   │   └── types.rs         # FFI 类型定义
│   │   │   │
│   │   │   ├── search_engine/       # 搜索引擎核心
│   │   │   │   ├── manager.rs       # Tantivy 管理器
│   │   │   │   ├── boolean_query_processor.rs
│   │   │   │   ├── highlighting_engine.rs
│   │   │   │   ├── concurrent_search.rs
│   │   │   │   ├── roaring_index.rs # 位图索引
│   │   │   │   └── ...
│   │   │   │
│   │   │   ├── archive/             # 多格式归档处理
│   │   │   │   ├── processor.rs     # 主处理器
│   │   │   │   ├── extraction_engine.rs
│   │   │   │   ├── extraction_orchestrator.rs
│   │   │   │   ├── fault_tolerance/ # 熔断器与自愈
│   │   │   │   ├── streaming/       # 流式解压
│   │   │   │   ├── actors/          # Actor 模型
│   │   │   │   └── ...
│   │   │   │
│   │   │   ├── storage/             # 存储层
│   │   │   │   ├── cas.rs           # 内容寻址存储
│   │   │   │   ├── metadata_store.rs# 元数据存储
│   │   │   │   ├── metrics_store.rs # 指标存储
│   │   │   │   └── integrity.rs     # 完整性验证
│   │   │   │
│   │   │   ├── services/            # 领域服务层
│   │   │   │   ├── pattern_matcher.rs  # Aho-Corasick
│   │   │   │   ├── query_executor.rs   # 查询执行
│   │   │   │   ├── query_planner.rs    # 查询计划
│   │   │   │   ├── fuzzy_matcher.rs    # 模糊匹配
│   │   │   │   ├── file_change_detector.rs # 变更检测
│   │   │   │   └── ...
│   │   │   │
│   │   │   ├── security/            # 安全模块
│   │   │   │   ├── import_security.rs
│   │   │   │   └── line_guard.rs
│   │   │   │
│   │   │   ├── task_manager/        # 异步任务 Actor 模型
│   │   │   ├── monitoring/          # 观测性与指标
│   │   │   ├── state_sync/          # 状态同步
│   │   │   ├── events/              # 事件系统
│   │   │   ├── models/              # 数据模型
│   │   │   ├── utils/               # 工具函数
│   │   │   ├── error.rs             # 错误处理
│   │   │   ├── lib.rs               # 库入口
│   │   │   └── main.rs              # 应用入口
│   │   │
│   │   ├── crates/                  # 本地 crates
│   │   │   ├── log-lexer/           # LogLexer 核心库
│   │   │   └── log-lexer-derive/    # 过程宏
│   │   │
│   │   ├── tests/                   # 集成测试
│   │   │   ├── archive_integration_tests.rs
│   │   │   ├── search_integration_tests.rs
│   │   │   ├── cas_migration_property_tests.rs
│   │   │   └── ...
│   │   │
│   │   ├── benches/                 # 性能基准测试
│   │   │   └── m1_benchmark.rs
│   │   │
│   │   ├── config/                  # 配置文件
│   │   └── capabilities/            # Tauri 权限配置
│   │
│   └── coverage/                    # 测试覆盖率报告
│
├── log-analyzer_flutter/            # Flutter 项目（实验性）
│   ├── lib/
│   │   ├── main.dart                # 应用入口
│   │   ├── core/                    # 核心模块
│   │   │   ├── constants/           # 常量定义
│   │   │   ├── router/              # 路由配置
│   │   │   ├── sentry/              # 错误追踪
│   │   │   └── theme/               # 主题配置
│   │   ├── features/                # 功能模块
│   │   │   ├── keyword/             # 关键词管理
│   │   │   ├── performance/         # 性能监控
│   │   │   ├── search/              # 搜索功能
│   │   │   ├── settings/            # 设置
│   │   │   ├── task/                # 任务管理
│   │   │   └── workspace/           # 工作区
│   │   ├── shared/                  # 共享模块
│   │   │   ├── models/              # 数据模型
│   │   │   ├── providers/           # Riverpod Providers
│   │   │   ├── services/            # 服务层
│   │   │   │   ├── api_service.dart # HTTP API 客户端
│   │   │   │   ├── bridge_service.dart # FFI 桥接
│   │   │   │   └── generated/       # 生成的 FFI 代码
│   │   │   └── widgets/             # 共享组件
│   │   └── l10n/                    # 国际化
│   ├── test/                        # 测试
│   └── shaders/                     # GLSL 着色器
│
├── docs/                            # 项目文档
│   ├── architecture/                # 架构文档
│   │   ├── ADVANCED_SEARCH_FEATURES_EXPLANATION.md
│   │   ├── API.md
│   │   └── CAS_ARCHITECTURE.md
│   ├── guides/                      # 用户指南
│   │   ├── MULTI_KEYWORD_SEARCH_GUIDE.md
│   │   └── QUICK_REFERENCE.md
│   ├── development/                 # 开发指南
│   │   └── AGENTS.md
│   └── reports/                     # 技术报告
│
├── scripts/                         # 工具脚本
│   ├── validate-ci.sh
│   └── validate-release.sh
│
├── CHANGELOG.md                     # 更新日志
├── CLAUDE.md                        # AI 上下文文档
├── AGENTS.md                        # 开发代理指南
├── LICENSE                          # Apache 2.0 许可证
└── README.md                        # 本文件
```

### 核心模块详解

#### 后端核心模块

| 模块 | 功能 | 技术要点 |
|------|------|----------|
| `commands/search.rs` | 搜索命令 | 核心搜索逻辑，支持多模式匹配 |
| `commands/async_search.rs` | 异步搜索 | 可取消的异步搜索，流式结果 |
| `commands/http_api.rs` | HTTP API | axum 服务器，供 Flutter 调用 |
| `ffi/bridge.rs` | FFI 桥接 | flutter_rust_bridge 2.x 实现 |
| `search_engine/manager.rs` | 搜索引擎管理 | Tantivy引擎，<200ms响应 |
| `search_engine/concurrent_search.rs` | 并发搜索 | 流式处理，内存O(max_concurrent) |
| `archive/processor.rs` | 归档处理 | 7层递归解压，流式处理 |
| `archive/fault_tolerance/` | 熔断自愈 | 锁中毒恢复，异常隔离 |
| `storage/cas.rs` | 内容寻址存储 | SHA-256哈希，原子操作 |
| `services/pattern_matcher.rs` | 模式匹配 | Aho-Corasick O(n+m) |
| `services/fuzzy_matcher.rs` | 模糊匹配 | Levenshtein 距离算法 |
| `services/file_change_detector.rs` | 变更检测 | SHA-256 哈希比较 |
| `security/import_security.rs` | 导入安全 | 路径验证，炸弹检测 |

---

## 🧪 测试与质量

### 测试覆盖

#### Rust后端

```bash
cd log-analyzer/src-tauri

# 运行所有测试
cargo test --all-features

# 显示测试输出
cargo test -- --nocapture

# 运行属性测试
cargo test --all-features -- proptest

# 性能基准测试
cargo bench

# 代码格式化
cargo fmt

# 静态分析
cargo clippy --all-features --all-targets -- -D warnings
```

**测试指标**:
- **测试用例**: 534/535 通过（99.8%）
- **覆盖率**: 80%+
- **Clippy警告**: 0
- **属性测试**: Proptest 集成

#### React前端

```bash
cd log-analyzer

# 运行测试
npm test

# 监听模式
npm run test:watch

# 类型检查
npm run type-check

# 代码检查
npm run lint
```

**测试框架**: Jest + React Testing Library

### CI/CD检查清单

- ✅ `cargo fmt --check` - 代码格式
- ✅ `cargo clippy -- -D warnings` - 静态分析
- ✅ `cargo test --all-features` - 单元测试
- ✅ `npm run lint` - 前端检查
- ✅ `npm run type-check` - TypeScript检查
- ✅ `npm run build` - 构建验证
- ✅ `npm run tauri build` - 发布版本编译

---

## 📊 性能指标

### 搜索性能

| 指标 | 数值 | 说明 |
|------|------|------|
| 单关键词搜索 | <10ms | 平均响应延迟 |
| 多关键词搜索 | <50ms | 10个关键词 |
| 吞吐量 | 10,000+次/秒 | Aho-Corasick算法 |
| 缓存命中率 | 85%+ | LRU缓存优化 |
| Tantivy查询 | <200ms | 高级布尔查询 |

### 内存优化

| 优化项 | 优化前 | 优化后 | 提升 |
|--------|--------|--------|------|
| 并发搜索 | O(n) | O(max_concurrent) | 90%+ ↓ |
| GZ解压 | 99MB | 10MB | 89% ↓ |
| 批量查询 | 1000×10MB | 10MB | 99% ↓ |

### 存储优化

- **自动去重**: 节省磁盘空间30%+
- **查询性能**: SQLite FTS5索引，10倍性能提升
- **并发安全**: DashSet缓存，支持高并发

---

## 📖 文档

### 核心文档

- [CLAUDE.md](CLAUDE.md) - AI上下文文档（项目根目录）
- [CHANGELOG.md](CHANGELOG.md) - 完整更新日志
- [AGENTS.md](AGENTS.md) - 开发代理指南

### 架构文档

- [CAS架构详解](docs/architecture/CAS_ARCHITECTURE.md)
- [API接口文档](docs/architecture/API.md)
- [高级搜索功能](docs/architecture/ADVANCED_SEARCH_FEATURES_EXPLANATION.md)

### 用户指南

- [快速参考指南](docs/guides/QUICK_REFERENCE.md)
- [多关键词搜索指南](docs/guides/MULTI_KEYWORD_SEARCH_GUIDE.md)

### 开发指南

- [AI Agent指南](docs/development/AGENTS.md)

---

## 🗺️ 开发路线图

### ✅ 已完成（v0.0.143）

#### 核心功能
- ✅ 多格式压缩包支持（ZIP/RAR/GZ/TAR/7Z）
- ✅ Aho-Corasick 搜索引擎（性能提升 80%+）
- ✅ CAS 存储系统（自动去重，节省 30% 空间）
- ✅ Tantivy 搜索引擎（支持高级布尔查询，<200ms 响应）
- ✅ 虚拟滚动（百万级日志实时渲染）
- ✅ 插件安全沙箱（目录白名单 + ABI 版本强制验证）
- ✅ 熔断自愈系统（Circuit Breaker + Poisoned Lock Recovery）
- ✅ 实时文件监听与增量索引
- ✅ 国际化支持（中英文实时切换）

#### 新增特性
- ✅ **HTTP API 服务器**: axum 实现，127.0.0.1:8080，供 Flutter 调用
- ✅ **FFI 桥接支持**: flutter_rust_bridge 2.x，支持 Flutter FFI 调用
- ✅ **双前端架构**: React (Tauri) + Flutter (跨平台) 并行支持
- ✅ **搜索历史功能**: 保存最近 50 条，智能去重，工作区隔离
- ✅ **模糊搜索**: Levenshtein 距离算法，容忍拼写错误
- ✅ **智能单词边界**: 自动检测，零用户配置
- ✅ **文件类型过滤**: 三层检测策略（二进制/智能规则/白名单）

#### 增量索引优化（v0.0.140）
- ✅ 偏移量持久化：应用重启后从上次位置继续读取
- ✅ 索引实时更新：监听的新内容可立即搜索
- ✅ 智能变更检测：基于 SHA-256 哈希避免无效索引
- ✅ 删除文件处理：自动清理索引结果

#### 架构优化
- ✅ 领域驱动设计（DDD）分层架构
- ✅ 全面异步化：磁盘 I/O 与归档处理 100% 异步非阻塞
- ✅ 并发安全：原子 Metrics 注册，彻底消除死锁风险
- ✅ 深度路径防御：支持嵌套归档路径递归扫描
- ✅ 零 Panic 保证：生产代码 100% 清理 `unwrap/expect`
- ✅ 流式并发处理：内存占用与文件大小脱钩
- ✅ LogLexer 过程宏：日志解析 DSL

#### 质量保障
- ✅ 534/535 测试通过（99.8%覆盖）
- ✅ Clippy 零警告
- ✅ CI/CD 全流程验证

### 🚧 进行中

- [ ] Flutter 前端功能完善
- [ ] 前端单元测试扩展
- [ ] 性能监控仪表板增强

### 📅 短期计划（1-2个月）

- [ ] 高级搜索语法（字段搜索、时间范围、通配符）
- [ ] 导出增强（JSON、Excel格式）
- [ ] 插件系统（自定义解析器）
- [ ] 性能优化（大文件处理、索引压缩）

### 🌟 长期愿景（3-6个月）

- [ ] 分布式索引（多机协同）
- [ ] 机器学习（异常检测、趋势预测）
- [ ] 可视化增强（时间线、关系图、热力图）
- [ ] 云端同步（可选功能，端到端加密）

---

## ❓ 常见问题

### 功能相关

**Q: 支持哪些日志格式？**
A: 支持所有文本格式（`.log`, `.txt`等）和压缩格式（`.zip`, `.tar`, `.gz`, `.rar`, `.7z`等）。

**Q: 导入的日志存储在哪里？**
A: 工作区数据存储在应用数据目录：
- Windows: `%APPDATA%/com.joeash.log-analyzer/workspaces/`
- macOS: `~/Library/Application Support/com.joeash.log-analyzer/workspaces/`
- Linux: `~/.local/share/com.joeash.log-analyzer/workspaces/`

**Q: 如何删除工作区释放空间？**
A: 在应用中删除工作区会自动删除对应的CAS对象和元数据。

**Q: 支持实时监听文件变化吗？**
A: ✅ **支持！** 应用会自动监听文件变化，新增内容实时索引并推送。

**Q: 支持多少层嵌套压缩包？**
A: 支持最多 7 层嵌套（例如：ZIP→TAR→GZ→ZIP→TAR→GZ→LOG）。

**Q: Flutter 版本和 React 版本有什么区别？**
A: React 版本是主要桌面版本，功能最完整。Flutter 版本是实验性的跨平台版本，支持移动端。

### 性能相关

**Q: 搜索很慢怎么办？**
A:
- 首次搜索会建立缓存，后续会快很多
- 使用更具体的搜索词减少结果数量
- 利用关键词过滤功能精准搜索
- 启用高级查询语法（AND/OR/NOT）

**Q: 处理大文件时内存占用高？**
A:
- 应用使用流式处理，内存占用已优化
- GZ文件1MB阈值自动触发流式解压
- ZIP文件500MB硬限制防止内存炸弹
- 并发解压可配置最大并发数

### 系统相关

**Q: Windows上提示权限错误？**
A: 应用会自动处理只读文件和UNC路径。如仍有问题，请以管理员身份运行。

**Q: macOS构建失败？**
A: 确保安装了Xcode Command Line Tools：`xcode-select --install`

**Q: Linux上依赖缺失？**
A: 参考 [Tauri Prerequisites](https://tauri.app/v1/guides/getting-started/prerequisites) 安装系统依赖。

**Q: 应用是否完全离线可用？**
A: ✅ **是的！** 应用设计为完全离线使用，所有数据处理在本地完成，无需网络连接。

**Q: HTTP API 端口可以修改吗？**
A: 目前 HTTP API 默认运行在 127.0.0.1:8080，未来版本将支持配置文件自定义。

---

## 🤝 贡献指南

欢迎贡献代码、报告问题或提出建议！

### 贡献流程

1. **Fork** 本仓库
2. 创建特性分支 (`git checkout -b feature/AmazingFeature`)
3. 提交更改 (`git commit -m 'Add AmazingFeature'`)
4. 推送到分支 (`git push origin feature/AmazingFeature`)
5. 开启 **Pull Request**

### 开发规范

- 遵循现有代码风格
- 添加测试用例覆盖新功能
- 更新相关文档
- 确保所有测试通过：
  - `cargo test --all-features`
  - `cargo clippy -- -D warnings`
  - `npm test`
  - `npm run lint`
  - `npm run type-check`
- 提交信息使用英文，清晰描述改动

### 编码原则

**必须使用业内成熟方案（铁律）**:

| 需求 | ✅ 推荐方案 | ❌ 禁止方案 |
|------|-----------|----------|
| 超时控制 | AbortController | 手写setTimeout + flag |
| 状态管理 | Zustand / React Query / Riverpod | 自造useState管理 |
| 多模式匹配 | Aho-Corasick算法库 | 逐行正则表达式 |
| 异步重试 | retry / tokio-retry | 手写loop + sleep |
| 表单验证 | Zod / Yup | 手写正则校验 |
| 日期处理 | date-fns / Day.js | moment.js（已过时） |
| HTTP客户端 | fetch / axios / dio | XMLHttpRequest原生 |
| 路由管理 | React Router / go_router | 自造hash路由 |
| FFI桥接 | flutter_rust_bridge | 手写FFI绑定 |

### 报告问题

使用 [Issues](https://github.com/ashllll/log-analyzer_rust/issues) 报告bug或提出功能建议时，请提供：

- 问题的详细描述
- 复现步骤
- 预期行为 vs 实际行为
- 系统环境（OS、版本号等）
- 相关日志或截图

---

## 📄 许可证

本项目采用 **Apache License 2.0** 开源协议。

详见 [LICENSE](LICENSE) 文件。

Copyright © 2024-2026 [ashllll](https://github.com/ashllll)

---

## 🙏 致谢

感谢以下开源项目：

- [Tauri](https://tauri.app/) - 跨平台桌面应用框架
- [Flutter](https://flutter.dev/) - 跨平台 UI 框架
- [flutter_rust_bridge](https://cjycode.com/flutter_rust_bridge/) - Flutter Rust FFI 桥接
- [Tantivy](https://github.com/quickwit-oss/tantivy) - 全文搜索引擎
- [aho-corasick](https://github.com/BurntSushi/aho-corasick) - 多模式字符串匹配
- [tokio](https://tokio.rs/) - 异步运行时
- [React](https://reactjs.org/) - 前端框架
- [Zustand](https://github.com/pmndrs/zustand) - 轻量级状态管理
- [TanStack Query](https://tanstack.com/query) - 强大的数据同步库
- [Zod](https://zod.dev/) - TypeScript优先的模式验证库
- [Riverpod](https://riverpod.dev/) - Flutter 状态管理

---

<div align="center">

**如果这个项目对您有帮助，请给个⭐Star！**

由 [ashllll](https://github.com/ashllll) 用 ❤️ 打造

[官网](https://github.com/ashllll/log-analyzer_rust) · [报告问题](https://github.com/ashllll/log-analyzer_rust/issues) · [功能建议](https://github.com/ashllll/log-analyzer_rust/issues)

</div>
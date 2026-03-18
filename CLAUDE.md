# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## 语言设置

**重要**: 本项目使用中文作为主要交流语言。请：

- 所有回答默认使用中文
- 代码注释使用中文
- 文档内容使用中文
- 仅在引用英文原文或技术术语时使用英文
- 所有答复使用中文回答。
- 任何情况下都不允许使用简单方案实施修复，不确定的问题先阅读代码或者搜索后在规划与实施修改。

***

> **项目**: log-analyzer\_rust - 高性能桌面日志分析工具
> **版本**: 0.0.138
> **技术栈**: Tauri 2.0 + Rust 1.70+ + React 19.1.0 + TypeScript 5.8.3
> **最后更新**: 2026-03-16

***

## 快速链接

- **[项目文档中心](docs/README.md)** - 架构文档、用户指南、开发指南
- **[Rust后端文档](log-analyzer/src-tauri/CLAUDE.md)** - 后端模块详细实现
- **[React前端文档](log-analyzer/src/CLAUDE.md)** - 前端模块详细实现

***

## 核心架构

### 技术栈

- **前端**: React 19.1.0 + TypeScript 5.8.3 + Zustand 5.0.9 + @tanstack/react-query 5.90.12 + Tailwind CSS 3.4.17
- **后端**: Rust 1.70+ + Tauri 2.0 + tokio 1.x + SQLite (sqlx 0.7)
- **搜索**: Aho-Corasick 算法 + Tantivy 0.22 全文搜索引擎 (性能提升 80%+)
- **存储**: 内容寻址存储(CAS) + SQLite + FTS5 全文搜索
- **插件系统**: libloading 动态库加载 + ABI 版本验证 + 目录白名单

### 应用数据目录 (离线存储)

- **Windows**: `%APPDATA%/com.joeash.log-analyzer/workspaces/`
- **macOS**: `~/Library/Application Support/com.joeash.log-analyzer/workspaces/`
- **Linux**: `~/.local/share/com.joeash.log-analyzer/workspaces/`

### 目录结构

```
log-analyzer_rust/
├── log-analyzer/
│   ├── src/                   # React前端
│   │   ├── components/        # UI组件 (ui/, modals/, renderers/, search/)
│   │   ├── pages/            # 页面(SearchPage, WorkspacesPage等)
│   │   ├── services/         # API封装、SearchQueryBuilder
│   │   ├── hooks/            # 自定义Hooks (useKeyboardShortcuts等)
│   │   ├── stores/           # Zustand状态管理
│   │   ├── types/            # TypeScript类型定义
│   │   └── i18n/             # 国际化翻译 (zh.json, en.json)
│   └── src-tauri/            # Rust后端
│       ├── src/
│       │   ├── application/   # 应用接入层 (plugins/, commands.rs)
│       │   ├── commands/     # Tauri命令(search, import, workspace等)
│       │   ├── search_engine/ # 搜索引擎(Tantivy,布尔查询,高亮引擎)
│       │   ├── services/     # 业务逻辑(PatternMatcher, QueryExecutor等)
│       │   ├── storage/      # CAS存储系统 + SQLite元数据
│       │   ├── archive/      # 压缩包处理(ZIP/RAR/GZ/TAR), 熔断自愈
│       │   ├── task_manager/ # 异步任务Actor模型
│       │   ├── monitoring/   # 观测性 (metrics, advanced)
│       │   └── models/       # 数据模型
│       └── tests/            # 集成测试
├── docs/                     # 项目文档
│   ├── architecture/         # 架构文档 (CAS, API, 搜索功能)
│   ├── guides/              # 用户指南 (快速参考, 多关键词搜索)
│   ├── development/         # 开发指南 (Agents, CI/CD)
│   └── reports/             # 项目报告
├── scripts/                  # 工具脚本
│   ├── validate-ci.sh       # CI验证脚本
│   └── validate-release.sh  # 发布验证脚本
└── CHANGELOG.md              # 更新日志
```

***

## 常用命令

### 环境要求

- **Node.js**: 22.12.0+ (通过 `engines` 字段强制)
- **npm**: 10.0+
- **Rust**: 1.70+ (MSVC 工具链 on Windows)
- **系统依赖**: [Tauri前置依赖](https://tauri.app/v1/guides/getting-started/prerequisites)
  - Linux: GTK3/GTK4 开发库 (根据系统已安装版本选择)
  - macOS: Xcode Command Line Tools
  - Windows: Microsoft C++ Build Tools + Windows SDK

### 开发

```bash
# 安装依赖
npm install

# 启动开发服务器
npm run tauri dev

# TypeScript类型检查
npm run type-check

# ESLint检查
npm run lint
npm run lint:fix

# 构建生产版本
npm run tauri build
```

### Rust测试

```bash
cd log-analyzer/src-tauri

# 运行所有测试
cargo test --all-features

# 显示测试输出
cargo test -- --nocapture

# 运行特定模块测试
cargo test pattern_matcher

# 运行集成测试
cargo test --test '*'

# 性能基准测试 (使用 criterion)
cargo bench

# 代码格式化
cargo fmt

# 代码格式检查 (CI使用)
cargo fmt -- --check

# 静态分析
cargo clippy -- -D warnings

# CI完整检查 (所有目标)
cargo clippy --all-features --all-targets -- -D warnings
```

### 前端测试

```bash
# 运行Jest测试
npm test

# 监听模式
npm run test:watch

# 生成覆盖率报告
npm test -- --coverage
```

***

## 核心开发任务

### 添加新的Tauri命令

**步骤**:

1. 在 `log-analyzer/src-tauri/src/commands/` 创建新文件(如 `my_feature.rs`)
2. 使用 `#[tauri::command]` 宏装饰函数
3. 在 `log-analyzer/src-tauri/src/commands/mod.rs` 中导出
4. 在 `log-analyzer/src-tauri/src/lib.rs` 的 `invoke_handler()` 中注册
5. 前端添加类型定义，使用 `invoke()` 调用

**注意事项**:

- 遵循「前后端集成规范」: 字段名必须一致 (task\_id 不是 taskId)
- 使用 `AppError` 进行错误处理
- 添加单元测试到命令文件中

### 调试Tauri IPC通信

1. **后端日志**: 使用 `tracing::{info, debug, error}` 添加日志
2. **前端日志**: 使用 `console.log/error` 检查调用结果
3. **DevTools**: 按 F12 打开开发者工具，查看 Console 和 Network
4. **序列化调试**: `println!("{}", serde_json::to_string_pretty(&my_data)?);`

**常见错误**:

- 字段名不一致: Rust `task_id` vs 前端 `taskId`
- Option/null 处理: Rust `None` → JSON `null`，但 Zod 不接受 `null`

### 添加新的前端页面

1. 创建页面组件
2. 添加 i18n 翻译 (zh.json / en.json)
3. 在导航中添加链接

**最佳实践**: 函数式组件 + Hooks，所有文案走 i18n，Tailwind Utility 类

***

## 前后端集成规范

> **关键**: Rust字段名 = JSON字段名 = TypeScript字段名

```rust
// Rust后端
#[derive(Serialize, Deserialize)]
pub struct TaskInfo {
    pub task_id: String;        // 直接用 snake_case
    pub task_type: String;
}
```

```typescript
// TypeScript前端
interface TaskInfo {
  task_id: string;              // 与Rust完全一致
  task_type: string;
}
```

### CAS存储 UNIQUE约束处理

```rust
// INSERT OR IGNORE + SELECT 模式
pub async fn insert_file(&self, metadata: &FileMetadata) -> Result<i64> {
    sqlx::query("INSERT OR IGNORE INTO files (...) VALUES (...)")
        .execute(&self.pool).await?;

    let id = sqlx::query_as::<_, (i64,)>("SELECT id FROM files WHERE sha256_hash = ?")
        .bind(&metadata.sha256_hash)
        .fetch_one(&self.pool).await?.0;

    Ok(id)
}
```

***

## 测试要求

### Rust后端

- **测试覆盖率**: 80%+
- **测试框架**: rstest (增强单元测试) + proptest (属性测试) + criterion (基准测试)
- **核心测试模块**:
  - `storage/`: CAS存储、完整性验证
  - `archive/`: 压缩包处理 (130+测试)
  - `search_engine/`: Tantivy搜索引擎、布尔查询、高亮引擎
  - `services/`: PatternMatcher、QueryExecutor、FileWatcher
  - `task_manager/`: Actor模型任务管理
  - `application/plugins/`: 插件安全验证

### React前端

- **测试框架**: Jest + React Testing Library
- **目标覆盖**: 80%+

***

## 代码质量检查

### 推送前强制验证 (Git Pre-Push Hook)

本项目已配置 **Git pre-push hook**，使用 Husky 管理 (`.husky/pre-push`)。

```bash
# 推送前自动执行
npm run validate:ci

# 或手动运行验证脚本
bash ../scripts/validate-ci.sh
```

### 验证内容

| 检查项           | 命令                                                         |
| ------------- | ---------------------------------------------------------- |
| ESLint        | `npm run lint`                                             |
| TypeScript 类型 | `npm run type-check`                                       |
| 前端测试          | `npm test -- --testPathIgnorePatterns=e2e`                 |
| 前端构建          | `npm run build`                                            |
| Rust 格式       | `cargo fmt -- --check`                                     |
| Rust Clippy   | `cargo clippy --all-features --all-targets -- -D warnings` |
| Rust 测试       | `cargo test --all-features --lib --bins`                   |

**完整CI验证**: `npm run validate:ci` (运行 `scripts/validate-ci.sh`)

### 跳过 Hook (不推荐)

```bash
git push --no-verify
```

***

## 编码规范

### 必须使用业内成熟方案（铁律）

| 需求      | 推荐方案                      | 禁止方案                    |
| ------- | ------------------------- | ----------------------- |
| 超时控制    | AbortController           | 手写setTimeout + flag     |
| 状态管理    | Zustand / React Query     | 自造useState管理            |
| 多模式匹配   | Aho-Corasick算法库           | 逐行正则表达式                 |
| 异步重试    | tokio-retry               | 手写loop + sleep          |
| 表单验证    | Zod / Validator derive    | 手写正则校验                  |
| 日期处理    | date-fns / chrono         | moment.js               |
| HTTP客户端 | fetch / reqwest           | XMLHttpRequest原生        |
| 全文搜索    | Tantivy                   | 手写倒排索引                  |
| 压缩处理    | zip/tar/flate2/async\_zip | 手写字节流解析                 |
| 错误处理    | thiserror / eyre / miette | String / Box<dyn Error> |

**例外**: 只有当不存在任何成熟方案时，经过用户明确批准才可实施自定义方案。

### Rust编码规范

- **命名**: `snake_case` (模块/函数), `CamelCase` (类型/Trait)
- **风格**: `cargo fmt`, `cargo clippy -- -D warnings`
- **错误处理**: 使用 `thiserror` 定义错误类型，使用 `eyre/miette` 进行错误报告
- **错误传播**: 使用 `?` 代替 `unwrap/expect` (生产代码100%消除panic)
- **文档注释**: 公开API添加 `///` 文档注释
- **并发安全**: 使用 `parking_lot` 高性能锁，`DashMap` 并发哈希映射
- **异步编程**: 使用 `tokio` 运行时，`async-trait` 异步trait

### TypeScript/React编码规范

- **命名**: `PascalCase` (组件/类型), `camelCase` (变量/函数)
- **组件**: 函数式组件 + Hooks，避免 class 组件
- **样式**: Tailwind Utility类 + `clsx` 条件类名 + `tailwind-merge` 合并
- **国际化**: 文案走 `i18next` 字典，使用 `useTranslation` Hook
- **类型安全**: 严格模式 TypeScript，避免 `any`
- **性能优化**: React.memo、useCallback、useMemo、虚拟滚动
- **状态管理**: Zustand (全局状态) + @tanstack/react-query (服务端状态)

***

## 故障排查

### 搜索无结果

1. 检查工作区状态是否为 `READY`
2. 查看后端日志确认索引已加载
3. 检查数据库: `SELECT COUNT(*) FROM files;`

### 任务一直显示"处理中"

- EventBus 版本号重复，幂等性跳过更新
- UNIQUE 约束冲突，任务未正常完成
- 确保任务事件版本号单调递增

### Windows路径过长错误

应用已使用 `dunce` crate 处理 UNC 路径。

### Linux GTK 依赖不匹配

```bash
# 检测已安装的 GTK 版本
pkg-config --modversion gtk+-3.0
pkg-config --modversion gtk4

# 根据返回版本号离线补齐对应包
```

***

## 核心架构决策

### 为什么选择 Aho-Corasick?

- 正则表达式逐行匹配复杂度 O(n×m)
- Aho-Corasick 多模式匹配复杂度 O(n+m)
- 性能提升 80%+，10,000+ 次搜索/秒

### 为什么采用 CAS 架构?

- 解决 Windows 260 字符路径限制
- SHA-256 内容寻址，自动去重
- 文件路径与内容解耦，节省磁盘空间 30%+

### QueryExecutor 职责拆分

- 拆分为 Validator、Planner、Executor 三个独立组件
- 符合单一职责原则(SRP)，代码复杂度降低 60%

### 插件系统安全架构

- **目录白名单**: 限制动态库加载路径
- **ABI版本验证**: 防止不兼容插件加载
- **深度路径防御**: 递归扫描防止路径遍历攻击
- **熔断自愈**: Circuit Breaker + Poisoned Lock Recovery

***

## 关键依赖说明

### Rust后端核心依赖 (Cargo.toml)

| 依赖             | 版本         | 用途                       |
| -------------- | ---------- | ------------------------ |
| `tauri`        | 2.0.0      | 跨平台桌面应用框架                |
| `tokio`        | 1.x (full) | 异步运行时                    |
| `tantivy`      | 0.22       | 全文搜索引擎 (Rust版Lucene)     |
| `aho-corasick` | 1.1        | 多模式字符串匹配算法               |
| `sqlx`         | 0.7        | 异步SQL工具包 (SQLite + FTS5) |
| `unrar`        | 0.5        | RAR格式支持 (libunrar绑定)     |
| `async_zip`    | 0.0.17     | 异步ZIP处理                  |
| `thiserror`    | 1.0        | 错误处理derive宏              |
| `eyre/miette`  | 0.6/5.0    | 结构化错误报告                  |
| `parking_lot`  | 0.12       | 高性能锁                     |
| `dashmap`      | 5.5        | 并发哈希映射                   |
| `tracing`      | 0.1        | 结构化日志追踪                  |
| `libloading`   | 0.8        | 动态库加载 (插件系统)             |
| `notify`       | 6.1        | 文件系统监听                   |

### React前端核心依赖 (package.json)

| 依赖                        | 版本       | 用途         |
| ------------------------- | -------- | ---------- |
| `react`                   | 19.1.0   | UI框架       |
| `@tauri-apps/api`         | 2.x      | Tauri前端API |
| `@tanstack/react-query`   | 5.90.12  | 服务端状态管理    |
| `@tanstack/react-virtual` | 3.13.12  | 虚拟滚动       |
| `zustand`                 | 5.0.9    | 客户端状态管理    |
| `framer-motion`           | 12.23.24 | 动画库        |
| `i18next`                 | 25.7.1   | 国际化        |

### 开发工具依赖

| 工具          | 用途          |
| ----------- | ----------- |
| `rstest`    | 增强单元测试      |
| `proptest`  | 属性测试 (模糊测试) |
| `criterion` | 性能基准测试      |
| `husky`     | Git hooks管理 |
| `jest`      | 前端测试框架      |

***

## 性能基准

### 搜索性能

- 单关键词搜索: <10ms
- 多关键词搜索(10个): <50ms
- Tantivy全文搜索: <200ms
- 吞吐量: 10,000+ 次搜索/秒

### 文件处理

- ZIP 解压: 100MB < 5秒 (500MB 硬限制防内存炸弹)
- GZ 流式解压: 1MB 阈值触发流式处理
- 索引构建: 10,000 行 < 1秒
- 增量更新: 1,000 行 < 100ms

### 内存优化

- 空闲状态: <100MB
- 加载 1GB 日志: <500MB
- 并发搜索内存: O(max\_concurrent) (非O(n))
- GZ解压峰值: 10MB (优化前99MB)

### 存储优化

- CAS自动去重: 节省磁盘空间 30%+
- SQLite FTS5: 查询性能提升10倍
- 索引压缩: Gzip压缩节省空间50%+

***

## 代码缺陷分析

> **分析日期**: 2026-03-16
> **分析范围**: 前端 React/TypeScript、后端 Rust、前后端通信、UI设计
> **缺陷总数**: 50 项（高危 12 / 中危 18 / 低危 20）

***

### 一、前端缺陷（React / TypeScript）

#### 高严重度

| #    | 缺陷描述                             | 文件                                           | 行号      |
| ---- | -------------------------------- | -------------------------------------------- | ------- |
| F-H1 | `any` 类型滥用，失去 TypeScript 类型保护    | `components/renderers/HybridLogRenderer.tsx` | 108-119 |
| F-H2 | `console.log/error` 生产环境日志泄露系统信息 | `App.tsx`                                    | 107-109 |
| F-H3 | `fetchNextPage()` 失败无 Toast 通知用户 | `pages/SearchPage.tsx`                       | 204-206 |
| F-H4 | API 响应强制类型转换，无 Zod Schema 验证     | `pages/SearchPage.tsx`                       | 623     |

#### 中严重度

| #    | 缺陷描述                                        | 文件                                                                      | 行号        |
| ---- | ------------------------------------------- | ----------------------------------------------------------------------- | --------- |
| F-M1 | 双 Toast 系统混用（自定义 Toast + react-hot-toast）   | `pages/SearchPage.tsx` & `stores/appStore.ts`                           | 多处        |
| F-M2 | 硬编码文本未国际化（中英混用）                             | `WorkspacesPage.tsx:49-50`、`SearchPage.tsx:667`、`ErrorBoundary.tsx:137` | 多处        |
| F-M3 | `useEffect` 依赖数组不完整导致闭包陷阱                   | `pages/SearchPage.tsx`                                                  | 486       |
| F-M4 | 搜索输入框和删除按钮缺少 `aria-label` 标签                | `pages/SearchPage.tsx`                                                  | 1062-1078 |
| F-M5 | 大对象 `currentQuery` 作 `useCallback` 依赖，频繁重渲染 | `pages/SearchPage.tsx`                                                  | 723       |
| F-M6 | 异步注册会话后组件可能已卸载（竞态条件）                        | `pages/SearchPage.tsx`                                                  | 419-427   |

#### 低严重度

| #     | 缺陷描述                                                  | 文件                                                 | 行号        |
| ----- | ----------------------------------------------------- | -------------------------------------------------- | --------- |
| F-L1  | 启用关键词组过滤逻辑重复（应提取共享）                                   | `SearchPage.tsx:137` & `HybridLogRenderer.tsx:108` | -         |
| F-L2  | `JSON.stringify(errorLogs)` 可能触发循环引用异常                | `components/ErrorBoundary.tsx`                     | 109-112   |
| F-L3  | 大量 `logger.debug()` 在生产环境执行影响性能                       | `hooks/useWorkspaceOperations.ts`                  | 多处        |
| F-L4  | 列表渲染使用 index 作 `key`，重排时状态错乱                          | `pages/SearchPage.tsx`                             | 1062-1080 |
| F-L5  | 未使用状态变量 `importStatus` 残留                             | `App.tsx`                                          | 65        |
| F-L6  | `localStorage.getItem()` 无 try-catch（隐私模式崩溃）          | `components/ErrorBoundary.tsx`                     | 64-77     |
| F-L7  | `SearchParams extends Record<string, unknown>` 破坏类型安全 | `services/api.ts`                                  | 58-62     |
| F-L8  | 事件 `payload` 强制类型转换，无运行时验证                            | `pages/SearchPage.tsx`                             | 437-438   |
| F-L9  | 工作区 ID 用 `Date.now()` 生成，高并发可能重复                      | `hooks/useWorkspaceOperations.ts`                  | 39        |
| F-L10 | 主内容区域缺少 `ErrorBoundary` 包裹                            | `App.tsx`                                          | 143-244   |

***

### 二、后端缺陷（Rust）

#### 高严重度

| #    | 缺陷描述                                             | 文件                              | 行号       |
| ---- | ------------------------------------------------ | ------------------------------- | -------- |
| B-H1 | `serde_json::to_string().unwrap()` 在测试关键路径 panic | `commands/virtual_tree.rs`      | 333, 348 |
| B-H2 | `strip_prefix().unwrap_or("")` 隐藏逻辑假设            | `services/metadata_db.rs`       | 76       |
| B-H3 | `blocking_lock()` 在 tokio 异步上下文中阻塞 worker 线程     | `commands/performance.rs`       | 52       |
| B-H4 | 路径遍历防护不完整：符号链接可绕过 `canonicalize()` 检查            | `archive/extraction_service.rs` | 323-335  |

#### 中严重度

| #    | 缺陷描述                                                     | 文件                        | 行号                                       | <br />                   | <br /> |
| ---- | -------------------------------------------------------- | ------------------------- | ---------------------------------------- | :----------------------- | :----- |
| B-M1 | 后台 `spawn` 任务中 `unwrap()` panic，前端无感知                    | `commands/search.rs`      | 289                                      | <br />                   | <br /> |
| B-M2 | watcher 回调线程长期持有锁，存在死锁风险                                 | `commands/watch.rs`       | 104-114, 149-156                         | <br />                   | <br /> |
| B-M3 | 使用 `std::sync::Mutex` 而非 `parking_lot::Mutex`（poison 问题） | `commands/search.rs`      | 935-940                                  | <br />                   | <br /> |
| B-M4 | \`Lazy::new(                                             | <br />                    | Regex::new(...).unwrap())\` 首次访问可能 panic | `commands/validation.rs` | 104    |
| B-M5 | 行号计算逻辑错误：`offset / 100` 不反映真实行数                          | `commands/watch.rs`       | 119-123                                  | <br />                   | <br /> |
| B-M6 | `MetadataDB` 线性 O(n) 遍历，大工作区性能瓶颈                         | `services/metadata_db.rs` | 71-81, 115-126                           | <br />                   | <br /> |

#### 低严重度

| #    | 缺陷描述                                        | 文件                                 | 行号                        |
| ---- | ------------------------------------------- | ---------------------------------- | ------------------------- |
| B-L1 | `unwrap_or()` 无 `warn!()` 日志，调试时无法定位默认值触发时机 | `commands/search.rs`               | 108, 629, 971, 1019, 1298 |
| B-L2 | 测试文件大量 `unwrap()`/`expect()`，失败信息不友好        | `archive/` 测试文件                    | 多处                        |
| B-L3 | `partial_cmp().unwrap_or()` 存在 NaN 场景风险     | `search_engine/index_optimizer.rs` | 437, 195                  |
| B-L4 | 应用关闭时 SQLite 连接无显式 flush/close              | `commands/import.rs`               | 107-117                   |
| B-L5 | `to_string_lossy()` 静默丢失非 UTF-8 文件名字节       | `commands/watch.rs`                | 102-146                   |

***

### 三、前后端通信与 UI 设计缺陷

#### 高严重度

| #    | 缺陷描述                                                 | 文件                            | 行号      |
| ---- | ---------------------------------------------------- | ----------------------------- | ------- |
| C-H1 | `workspaceId == "default"` 静默降级到随机工作区，搜索结果来源错误       | `commands/search.rs`          | 298     |
| C-H2 | `invoke('search_logs')` 无 Zod Schema 验证响应，运行时崩溃风险    | `services/api.ts`             | 228-234 |
| C-H3 | 后端发送 `null`，前端 `null \|\| undefined` 转换不完整，工作区状态同步失败 | `stores/AppStoreProvider.tsx` | 131     |
| C-H4 | 未选择工作区时搜索按钮未禁用、无提示，用户操作无反馈                           | `pages/SearchPage.tsx`        | 389-394 |

#### 中严重度

| #    | 缺陷描述                                               | 文件                                         | 行号      |
| ---- | -------------------------------------------------- | ------------------------------------------ | ------- |
| C-M1 | 空 task ID 用 `task-${index}` 作 key，违反 React key 唯一性 | `pages/TasksPage.tsx`                      | 35-36   |
| C-M2 | 配置加载失败后无默认工作区兜底，应用整体不可用                            | `stores/AppStoreProvider.tsx`              | 46-66   |
| C-M3 | `SearchPage` 无"无工作区"、"无结果"空状态 UI                   | `pages/SearchPage.tsx`                     | 整体      |
| C-M4 | `FileFilterSettings` 快速打开/关闭无 AbortController，响应乱序 | `components/modals/FileFilterSettings.tsx` | 39-57   |
| C-M5 | 导出功能无 try-catch、无进度提示、无成功/失败反馈                     | `pages/SearchPage.tsx`                     | 940-950 |
| C-M6 | `task-update` 事件幂等性不完整，重复处理导致多次 Toast              | `stores/AppStoreProvider.tsx`              | 78-110  |

#### 低严重度

| #    | 缺陷描述                                                   | 文件                                   | 行号    |
| ---- | ------------------------------------------------------ | ------------------------------------ | ----- |
| C-L1 | `SearchParams` 用 camelCase 定义，与 Rust snake\_case 约定不一致 | `services/api.ts`                    | 58-72 |
| C-L2 | `Suspense` 未被 `ErrorBoundary` 包裹，lazy load 失败无处理       | `App.tsx`                            | 31-36 |
| C-L3 | `PerformancePage` 多个独立 `isLoading` 状态，界面闪烁             | `pages/PerformancePage.tsx`          | 整体    |
| C-L4 | `KeywordModal` 正则表达式无实时验证提示，可保存无效配置                    | `components/modals/KeywordModal.tsx` | 整体    |
| C-L5 | 工作区删除无确认对话框，用户可能误删                                     | `pages/WorkspacesPage.tsx`           | 98    |

***

### 修复优先级

#### 第一阶段 — 高危（立即修复）

1. **C-H1** 修复搜索工作区 ID 降级逻辑 (`search.rs:298`)
2. **B-H3** 修复 `blocking_lock()` 阻塞 tokio worker (`performance.rs:52`)
3. **B-H4** 路径遍历：禁用符号链接跟踪 (`extraction_service.rs:323-335`)
4. **C-H4** 无工作区时禁用搜索按钮并显示提示 (`SearchPage.tsx:389-394`)
5. **F-H4** 添加 Zod Schema 验证 API 响应 (`SearchPage.tsx:623`)

#### 第二阶段 — 中危（本周内）

1. **B-M5** 修复行号计算逻辑 (`watch.rs:119-123`)
2. **B-M6** MetadataDB 添加反向索引 (`metadata_db.rs:71-81`)
3. **C-M2** 配置加载失败时设置默认工作区 (`AppStoreProvider.tsx:46-66`)
4. **C-M3** SearchPage 实现完整空状态 UI
5. **C-M4** FileFilterSettings 添加 AbortController (`FileFilterSettings.tsx:39-57`)

#### 第三阶段 — 低危（计划中）

1. **F-L4** 修复列表 key 使用 index 问题
2. **C-L5** 工作区删除添加确认对话框
3. **F-M2** 补全国际化翻译
4. **B-L1** 为 `unwrap_or()` 添加 `warn!()` 日志
5. **F-L6** localStorage 访问添加 try-catch

***


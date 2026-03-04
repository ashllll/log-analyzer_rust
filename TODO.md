# 未完成任务清单 (TODO)

> 本文档记录代码中所有未完成的任务、TODO 注释和待实现功能。
> 最后更新: 2026-02-21

---

## 🔧 Flutter + Rust HTTP API 集成 (2026-02-21)

> **状态**: ✅ 基础框架已完成 | ⚠️ 业务逻辑集成待完成

### 已完成的实现

| # | 文件 | 实现内容 | 状态 |
|---|------|----------|------|
| 1 | `Cargo.toml` | 添加 axum、tower、tower-http 依赖 | ✅ |
| 2 | `src/commands/http_api.rs` | 新建 HTTP API 适配层 | ✅ |
| 3 | `src/commands/mod.rs` | 导出 http_api 模块 | ✅ |
| 4 | `src/main.rs` | 启动 HTTP 服务器 (127.0.0.1:8080) | ✅ |
| 5 | `pubspec.yaml` | 添加 dio 依赖 | ✅ |
| 6 | `bridge_service.dart` | 使用 dio 实现 HTTP 客户端 | ✅ |
| 7 | `Cargo.toml` | 修复 lib crate-type | ✅ |

### 待完成的任务 (P0 - 高优先级)

#### 1. 🔴 HTTP API 业务逻辑集成
**位置**: `src/commands/http_api.rs`

**问题**: 当前 HTTP API 是简化实现，返回的是模拟数据。需要与实际业务逻辑集成：

- [ ] `search_logs` - 调用 `crate::commands::search::search_logs` 实际搜索逻辑
- [ ] `create_workspace` - 调用 `crate::commands::workspace::create_workspace` 创建工作区
- [ ] `delete_workspace` - 调用 `crate::commands::workspace::delete_workspace` 删除工作区
- [ ] `refresh_workspace` - 调用 `crate::commands::workspace::refresh_workspace` 刷新工作区
- [ ] `load_config` - 调用 `crate::commands::config::load_config` 加载配置
- [ ] `save_config` - 调用 `crate::commands::config::save_config` 保存配置

**难度**: 需要解决 AppState 共享问题（HTTP 服务器运行在独立线程）

#### 2. 🔴 HTTP API 路由扩展
**位置**: `src/commands/http_api.rs`

需要添加更多 API 端点以支持完整功能：

- [ ] `GET /api/keywords` - 获取关键词列表
- [ ] `POST /api/keywords` - 添加关键词组
- [ ] `PUT /api/keywords/:id` - 更新关键词组
- [ ] `DELETE /api/keywords/:id` - 删除关键词组
- [ ] `POST /api/watch/start` - 启动文件监听
- [ ] `POST /api/watch/stop` - 停止文件监听
- [ ] `POST /api/import/folder` - 导入文件夹
- [ ] `GET /api/performance/metrics` - 获取性能指标

#### 3. 🟡 错误处理完善
**位置**: `src/commands/http_api.rs`

- [ ] 添加统一的错误处理中间件
- [ ] 正确映射 `AppError` 到 HTTP 状态码
- [ ] 添加请求日志和调试支持

#### 4. 🟡 安全性增强
**位置**: `src/commands/http_api.rs`

- [ ] 添加请求频率限制（防止滥用）
- [ ] 添加 API 密钥认证（可选）
- [ ] 生产环境禁用 CORS `allow_origin(Any)`

---

## 🔧 PRD V6.0 二次深入分析 - 编译修复 (2026-02-15)

> **状态**: ✅ 编译通过 (0 错误) | ⚠️ 5 个警告待清理

### 已修复的编译错误

| # | 文件 | 问题 | 解决方案 |
|---|------|------|----------|
| 1 | `crates/log-lexer-derive/src/lib.rs` | `syn::TokenTree` 在 syn 2.0 中被移除 | 替换为 `proc_macro2::TokenTree` |
| 2 | `src/search_engine/roaring_index.rs` | roaring bitmap 使用 u32，代码使用 u64 | 添加类型转换 |
| 3 | `src/search_engine/streaming_builder.rs` | content 字段 `Arc<str>` 类型不匹配 | 使用 `.into()` 转换 |
| 4 | `src/services/file_watcher_async.rs` | `next_line()` 返回类型变更 | 使用 `while let Ok(Some(line))` 模式 |
| 5 | `src/utils/encoding_detector.rs` | `mime_name()` 方法不存在 | 使用 `name()` 替代 |
| 6 | `src/utils/transcoding_pipe.rs` | `decode_to_utf8` 返回值变更 | 更新元组解构 |
| 7 | `src/search_engine/dfa_engine.rs` | `states()` 方法私有 | 使用启发式估计 |
| 8 | `Cargo.toml` | roaring 缺少 serde feature | 添加 `features = ["serde"]` |

### 修复详情

#### 1. log-lexer-derive 编译修复
```rust
// 问题: syn 2.0 移除了 TokenTree
// 修复前: syn::TokenTree
// 修复后: proc_macro2::TokenTree
```

#### 2. roaring_index.rs 类型修复
```rust
// 问题: roaring bitmap 仅支持 u32，但代码使用 u64
// 修复: 添加 as u32 类型转换
```

#### 3. streaming_builder.rs Arc<str> 修复
```rust
// 问题: content 字段需要 Arc<str> 类型
// 修复: 使用 .into() 进行 From<&str> -> Arc<str> 转换
```

#### 4. file_watcher_async.rs 异步 API 修复
```rust
// 问题: next_line() 返回 Result<Option<String>>
// 修复: 使用 while let Ok(Some(line)) = reader.next_line().await
```

#### 5. encoding_detector.rs API 修复
```rust
// 问题: chardetng 的 EncodingExt::mime_name() 不存在
// 修复: 使用 encoding.name() 替代
```

#### 6. transcoding_pipe.rs decode API 修复
```rust
// 问题: decode_to_utf8 返回 (Cow<str>, bool) 元组
// 修复: 更新元组解构模式
```

#### 7. dfa_engine.rs 私有方法修复
```rust
// 问题: regex_automata 的 DFA states() 方法是私有的
// 修复: 使用启发式估计 state_count = pattern_len * 3
```

#### 8. Cargo.toml 依赖修复
```toml
# 修复前
roaring = "0.10"

# 修复后
roaring = { version = "0.10", features = ["serde"] }
```

---

## 🎉 PRD V6.0 实施完成 (2026-02-15)

> **重要里程碑**: PRD V6.0 "Rustacean Architecture" 全部 10 个任务已完成！

### M1: Rustacean 底座层 ✅

| 任务 | 负责人 | 核心实现 | 状态 |
|------|--------|----------|------|
| M1.1 Typestate FFI 集成 | rust-backend-1 | Session 生命周期管理 (Unmapped→Mapped→Indexed), PageManager | ✅ |
| M1.2 Roaring Bitmap 压缩 | rust-backend-2 | 搜索结果位图压缩，千万级结果 < 5MB | ✅ |
| M1.3 Benchmark 测试 | rust-backend-4 | criterion 性能测试框架，7 个测试组 | ✅ |

**关键文件**:
- `src/ffi/commands_bridge.rs` - FFI 命令桥接
- `src/ffi/global_state.rs` - 全局状态管理
- `src/services/typestate/` - Typestate 模式实现
- `benches/m1_benchmark.rs` - 性能基准测试

### M2: 渲染层 ✅

| 任务 | 负责人 | 核心实现 | 状态 |
|------|--------|----------|------|
| M2.1 Flutter 虚拟滚动 | flutter-frontend-1 | SliverFixedExtentList, 确定性视口 | ✅ |
| M2.2 GLSL Shader 热力图 | flutter-frontend-1 | FragmentProgram GPU 着色器 | ✅ |

### M3: 搜索引擎 ✅

| 任务 | 负责人 | 核心实现 | 状态 |
|------|--------|----------|------|
| M3.1 DFA 正则引擎 | rust-backend-2 | regex-automata + Roaring Bitmap 集成 | ✅ |
| M3.2 LogLexer 过程宏 | rust-backend-5 | 过程宏实现，SIMD 词法分析 | ✅ |

**关键文件**:
- `src/search_engine/roaring_index.rs` - Roaring Bitmap 索引
- `crates/log-lexer/` - LogLexer crate
- `crates/log-lexer-derive/` - 过程宏

### M4: 容灾层 ✅

| 任务 | 负责人 | 核心实现 | 状态 |
|------|--------|----------|------|
| M4.1 编码嗅探与转码 | rust-backend-3 | chardetng 编码探测 + UTF-8 转码管道 | ✅ |
| M4.2 JSON 炸弹防护 | rust-backend-4 | LineGuard (1MB 限制) + ImportSecurity | ✅ |
| M4.3 logrotate 重连 | rust-backend-5 | Inode 追踪 + 文件句柄重连 | ✅ |

**关键文件**:
- `src/security/line_guard.rs` - 单行防护器
- `src/security/import_security.rs` - 导入安全检查
- `src/infrastructure/encoding/` - 编码处理模块

### PRD V6.0 SLA 性能指标

| 指标 | 目标 | 验证方式 |
|------|------|----------|
| FFI 视口拉取延迟 | < 1ms | `cargo bench --bench m1_benchmark --features ffi` |
| 全量并发盲搜吞吐量 | 3-5GB/s | criterion search_throughput 测试 |
| 10GB 文件驻留内存 | < 50MB | PageManager 滑动窗口 |
| 千万级搜索结果压缩 | < 5MB | Roaring Bitmap 压缩 |

---

## 🎯 任务优先级说明

- **P0 - 高优先级**: 核心功能缺失，影响用户体验
- **P1 - 中优先级**: 性能优化或架构改进
- **P2 - 低优先级**: 代码清理或文档完善

---

## ✅ 已完成任务

### P0 - 高优先级

#### 1. ✅ 搜索历史记录功能 (2026-02-12 完成)
**位置**: `src/commands/search_history.rs`

**实现内容**:
- [x] 创建 `src/models/search_history.rs` - 数据模型和 SearchHistoryManager
- [x] 创建 `src/commands/search_history.rs` - Tauri 命令
- [x] 实现 `add_search_history` 命令
- [x] 实现 `get_search_history` 命令
- [x] 实现 `clear_search_history` 命令
- [x] 在 `AppState` 中添加 `search_history` 字段
- [x] 在 `main.rs` 中注册命令
- [x] 6 个单元测试全部通过

---

### P1 - 中优先级

#### 2. ✅ 任务管理器性能指标 (2026-02-12 完成)
**位置**: `src/commands/performance.rs`

**实现内容**:
- [x] 在 `TaskManager` 中添加 `get_metrics_async()` 公开方法
- [x] 使用 `tokio::task::block_in_place` + `tauri::async_runtime::block_on` 实现同步调用
- [x] 获取真实的任务统计数据 (total, running, completed, failed)
- [x] 正确处理 TaskManager 未初始化的情况

---

#### 3. ✅ 索引指标数据 (2026-02-12 完成)
**位置**: `src/commands/performance.rs`

**实现内容**:
- [x] 使用 MetadataStore 的 `count_files()` 方法获取文件数量
- [x] 使用 MetadataStore 的 `sum_file_sizes()` 方法获取文件大小
- [x] 聚合所有工作区的统计数据
- [x] 估算索引大小（约为原始数据 20%）

---

#### 4. ✅ 工作区名称读取 (2026-02-12 完成)
**位置**: `src/commands/workspace.rs`

**实现内容**:
- [x] 创建 `WorkspaceMetadata` 结构体
- [x] 在 `create_workspace` 时保存元数据到 `workspace.json`
- [x] 在 `get_workspace_status` 时从元数据读取实际名称
- [x] 提供后备方案：从 workspaceId 提取名称

---

### P2 - 低优先级（架构清理）

#### 5. ✅ DDD 架构模块 (2026-02-12 完成)
**位置**: `src/infrastructure/mod.rs`, `src/domain/mod.rs` 等

**已完成模块**:
- [x] `domain/log_analysis/services.rs` - 日志解析服务、分析服务、工作区分析服务
- [x] `domain/log_analysis/repositories.rs` - 仓储接口和实体定义
- [x] `domain/shared/specifications.rs` - 规格模式实现
- [x] `infrastructure/persistence.rs` - 工作区/关键词组/搜索历史仓储实现 (3个测试通过)
- [x] `infrastructure/messaging.rs` - 事件总线、领域事件发布器、事件重放器 (3个测试通过)
- [x] `infrastructure/external.rs` - 健康检查器、速率限制器(令牌桶)、服务管理器 (4个测试通过)

**剩余可选模块** (根据需求实现):
- `domain/shared/value_objects` - 通用值对象
- `domain/log_analysis/events` - 日志分析事件（已有 `domain/shared/events.rs`）

---

#### 6. ✅ 插件系统集成 (2026-02-13 完成)
**位置**: `src/application/services/mod.rs`

**实现内容**:
- [x] 在 `LogAnalysisService` 中集成 `PluginManager`
- [x] 实现 `analyze_log_file()` 中的插件处理
- [x] 实现 `search_logs()` 中的插件预处理查询
- [x] 添加 `initialize_plugins()`, `load_plugin()`, `unload_plugin()` 方法
- [x] 修复 `monitoring/metrics.rs` 中的错误类型转换
- [x] 682/683 测试通过

---

#### 7. ✅ 配置文件加载 (2026-02-13 完成)
**位置**: `src/infrastructure/config/mod.rs`

**实现内容**:
- [x] JSON 配置文件读取 (`serde_json`)
- [x] TOML 配置文件读取 (`toml` crate)
- [x] 配置验证 (`validator` crate)
- [x] 环境变量覆盖 (12-Factor App 模式)
- [x] 完整错误处理 (`ConfigError` 枚举)
- [x] 16 个单元测试全部通过

---

#### 8. ✅ OpenTelemetry 集成 (2026-02-13 完成)
**位置**: `src/monitoring/mod.rs`

**实现内容**:
- [x] 添加 `tracing-opentelemetry`, `opentelemetry`, `opentelemetry_sdk`, `opentelemetry-otlp` 依赖
- [x] 创建 `telemetry` feature flag 控制是否启用
- [x] 配置 OTLP exporter (支持 gRPC)
- [x] 支持环境变量配置 (`OTEL_EXPORTER_OTLP_ENDPOINT`, `OTEL_SERVICE_NAME`)
- [x] 不启用 telemetry feature 时零性能影响
- [x] 683 个测试全部通过

---

## ✅ Flutter 前端已完成任务

> **注意**: 项目已从 React 迁移到 Flutter，Flutter 前端位于 `log-analyzer_flutter/` 目录

### P0 - 高优先级

#### 1. ✅ Flutter Provider API 调用 (2026-02-13 完成)
**位置**: `log-analyzer_flutter/lib/shared/providers/`

**实现内容**:
- [x] `workspace_provider.dart` - 工作区 CRUD 操作
- [x] `keyword_provider.dart` - 关键词组 CRUD 操作
- [x] `app_provider.dart` - 初始化 FFI 桥接和加载配置
- [x] `task_provider.dart` - 定期清理过期任务（TTL 机制）
- [x] 避免循环依赖架构设计
- [x] flutter analyze 通过

---

#### 2. ✅ Flutter 事件流服务集成 (2026-02-13 完成)
**位置**: `log-analyzer_flutter/lib/shared/services/event_stream_service.dart`

**实现内容**:
- [x] flutter_rust_bridge 集成
- [x] 事件解析和分发（6种事件类型：SearchResults, SearchSummary, TaskUpdate, FileChanged, WorkspaceStatus, SystemEvent）
- [x] Riverpod Providers（connectionStatus, taskUpdate, searchResults, systemEvent, eventError）
- [x] 轮询模式 + 外部事件注入接口
- [x] flutter analyze 通过

---

### P1 - 中优先级

#### 3. ✅ Flutter 关键词页面功能 (2026-02-13 完成)
**位置**: `log-analyzer_flutter/lib/features/keyword/presentation/keywords_page.dart`

**实现内容**:
- [x] isLoading 状态获取（从 keywordLoadingProvider）
- [x] 拖拽排序功能（ReorderableListView）
- [x] 文件选择和导入（file_picker + JSON 解析）
- [x] 文件保存和导出（JSON 格式）
- [x] 编辑功能（编辑对话框）
- [x] 复制功能（创建副本）
- [x] flutter analyze 通过

---

#### 4. ✅ Flutter 搜索页面功能 (2026-02-13 完成)
**位置**: `log-analyzer_flutter/lib/features/search/presentation/search_page.dart`

**实现内容**:
- [x] 虚拟滚动（flutter_virtual_scroll，支持 10,000+ 条日志）
- [x] 事件流监听（searchResults/searchSummary 流）
- [x] 导出对话框（JSON/CSV 格式，带时间戳文件名）
- [x] 过滤器应用逻辑（时间范围、日志级别、文件模式）
- [x] flutter analyze 通过

---

#### 5. ✅ Flutter Sentry 错误追踪集成 (2026-02-13 完成)
**位置**: `log-analyzer_flutter/lib/core/sentry/sentry_config.dart`

**实现内容**:
- [x] `SentryConfig` - 配置常量和环境判断
- [x] `SentryInitializer` - 初始化器（含敏感信息过滤）
- [x] `SentryUtils` - 便捷工具方法（captureException, setUser, addBreadcrumb）
- [x] `ErrorCapture` - 错误捕获包装器（wrapAsync, wrapSync）
- [x] 仅 Release 模式启用，DSN 通过 `--dart-define=SENTRY_DSN` 配置
- [x] 敏感信息过滤（Authorization, Cookie, password, token 等）

---

### P2 - 低优先级

#### 6. ✅ Flutter 输入组件状态管理 (2026-02-13 完成)
**位置**: `log-analyzer_flutter/lib/shared/widgets/custom_input.dart`

**实现内容**:
- [x] 将 `CustomInput` 从 `StatelessWidget` 改为 `StatefulWidget`
- [x] 修复 TextEditingController 生命周期管理问题（内存泄漏）
- [x] 在 `dispose()` 中正确释放内部 controller
- [x] 添加 `onSubmitted` 回调参数支持表单提交
- [x] 确认其他 4 个文件的 controller 管理正确

---

#### 7. ✅ Flutter 任务页面过滤功能 (2026-02-13 完成)
**位置**: `log-analyzer_flutter/lib/features/task/presentation/tasks_page.dart`

**实现内容**:
- [x] `TaskFilterType` 枚举（全部、运行中、已完成、失败、已停止）
- [x] 过滤菜单 UI（带数量显示）
- [x] 选中状态指示器
- [x] 过滤图标高亮
- [x] 空状态差异化提示
- [x] 快速重置按钮
- [x] SnackBar 反馈
- [x] flutter analyze 通过

---

#### 9. ✅ DDD 通用值对象模块 (2026-02-14 完成)
**位置**: `src/domain/shared/value_objects.rs`

**实现内容**:
- [x] `NonEmptyString` - 非空字符串值对象
- [x] `BoundedString` - 有长度限制的字符串值对象
- [x] `Email` - 电子邮件地址值对象（带格式验证）
- [x] `Url` - URL 地址值对象（支持 http/https/ftp/file 协议）
- [x] `FilePath` - 文件路径值对象（带路径遍历攻击检测）
- [x] `PositiveInteger` - 正整数值对象
- [x] `ValueError` - 统一的验证错误类型
- [x] 31 个单元测试全部通过
- [x] 跨平台兼容性（Windows/Linux）

---

#### 10. ✅ Flutter 导航观察器 Sentry 集成 (2026-02-14 确认已完成)
**位置**: `log-analyzer_flutter/lib/core/router/app_router.dart`

**实现内容**:
- [x] 导入 `sentry_flutter` 包
- [x] 在 `GoRouter` 配置中添加 `SentryNavigatorObserver()`
- [x] 自动追踪页面导航事件

---

#### 11. ✅ DDD 搜索领域模块 (2026-02-14 完成)
**位置**: `src/domain/search/`

**实现内容**:
- [x] `value_objects.rs` - SearchQuery, SearchMode, SearchPriority 值对象
- [x] `entities.rs` - SearchResult, SearchSession 实体
- [x] `services.rs` - SearchStrategy trait, SearchAggregator, ExactMatchStrategy, FuzzyMatchStrategy
- [x] `repositories.rs` - SearchRepository trait, InMemorySearchRepository
- [x] 21 个单元测试全部通过

---

#### 12. ✅ DDD 导出领域模块 (2026-02-14 完成)
**位置**: `src/domain/export/`

**实现内容**:
- [x] `value_objects.rs` - ExportFormat 枚举, ExportOptions 配置
- [x] `entities.rs` - ExportTask, ExportTaskStatus, ExportResult 实体
- [x] `services.rs` - ExportStrategy trait, ExportAggregator, JsonExportStrategy, CsvExportStrategy, TextExportStrategy
- [x] `repositories.rs` - ExportRepository trait, InMemoryExportRepository, ExportStorageStats
- [x] 22 个单元测试全部通过

---

#### 13. ✅ CQRS 查询处理器模块 (2026-02-14 完成)
**位置**: `src/application/queries/`

**实现内容**:
- [x] `queries.rs` - GetWorkspaceQuery, SearchLogsQuery, GetKeywordsQuery, GetTaskStatusQuery
- [x] `handlers.rs` - QueryHandler trait, QueryResult, QueryResponse 类型
- [x] `bus.rs` - QueryBus 类型擦除实现，支持异步查询分发
- [x] Send 线程安全修复（RwLock 守卫跨 await 点问题）

---

#### 14. ✅ CQRS 命令处理器模块 (2026-02-14 完成)
**位置**: `src/application/handlers/`

**实现内容**:
- [x] `commands.rs` - CreateWorkspaceCommand, ImportFilesCommand, DeleteWorkspaceCommand, SaveKeywordsCommand, CancelTaskCommand
- [x] `handlers.rs` - CommandHandler trait, CommandResult 类型
- [x] `bus.rs` - CommandBus 实现（移除了不兼容 dyn 的 middleware）
- [x] 零 clippy 警告，811 个测试全部通过

---

## 📋 可选后续任务

### P2 - 低优先级（根据需求实现）

> 目前没有待实现的可选任务。所有规划的功能模块已完成。

---

## 📊 统计摘要

| 类别 | 数量 | 详情 |
|------|------|------|
| **PRD V6.0 里程碑** | 10 项 | M1: 3, M2: 2, M3: 2, M4: 3 |
| **历史已完成 TODO** | 21 项 | P0: 3, P1: 6, P2: 12 |
| **HTTP API 框架** | 7 项 | ✅ 已完成 |
| **HTTP API 待集成** | 8+ 项 | P0: 业务逻辑集成, P1: 路由扩展 |
| **Rust 后端待办** | 8 项 | HTTP API 业务逻辑集成 |
| **Flutter 前端待办** | 0 项 | - |

---

## 🔗 相关链接

- [开发指南](docs/development/AGENTS.md)
- [Rust 后端文档](log-analyzer/src-tauri/CLAUDE.md)
- [Flutter 前端文档](log-analyzer_flutter/CLAUDE.md)
- [CHANGELOG](CHANGELOG.md)

---

> **注意**: 本文档会随着代码变更持续更新。在实施任务前请先检查最新状态。

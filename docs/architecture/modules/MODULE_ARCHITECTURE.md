# 模块架构完整文档

> 基于2026-03-28代码分析生成 | 版本: 1.2.53

本文档通过直接分析项目代码生成，不依赖已有文档内容。

---

## 目录

1. [Rust后端模块](#rust后端模块)
2. [React前端模块](#react前端模块)
3. [模块依赖关系](#模块依赖关系)
4. [架构分层](#架构分层)

---

## Rust后端模块

### 顶层模块结构

基于 `src-tauri/src/lib.rs` 分析：

```
src-tauri/src/
├── application/      # 应用接入层（DDD应用层）
├── archive/          # 压缩包处理（27个模块）
├── commands/         # Tauri IPC命令（15个模块）
├── domain/           # 领域层（DDD核心）
├── error.rs          # 统一错误处理
├── events/           # 事件系统（EventBus）
├── infrastructure/   # 基础设施层
├── lib.rs            # 库入口
├── models/           # 数据模型（15个模块）
├── monitoring/       # 监控和可观测性
├── search_engine/    # 搜索引擎（11个模块）
├── services/         # 业务服务（19个模块）
├── state_sync/       # 状态同步
├── storage/          # CAS存储（7个模块）
├── task_manager/     # 任务管理器（Actor模型）
├── main.rs           # 应用入口
└── utils/            # 工具函数（12个模块）
```

---

### 1. commands/ - Tauri IPC命令层

**位置**: `src-tauri/src/commands/mod.rs`

提供前端调用的所有Tauri命令接口。

| 模块文件 | 功能描述 |
|---------|---------|
| `async_search.rs` | 异步搜索命令 |
| `cache.rs` | 缓存管理命令 |
| `config.rs` | 配置管理命令 |
| `error_reporting.rs` | 错误上报命令 |
| `export.rs` | 导出结果命令 |
| `import.rs` | 文件导入命令 |
| `legacy.rs` | 遗留兼容命令 |
| `performance.rs` | 性能监控命令 |
| `query.rs` | 结构化查询命令 |
| `search.rs` | 同步搜索命令 |
| `state_sync.rs` | 状态同步命令 |
| `validation.rs` | 验证命令 |
| `virtual_tree.rs` | 虚拟文件树命令 |
| `watch.rs` | 文件监听命令 |
| `workspace.rs` | 工作区管理命令 |

**关键导出**:
```rust
pub mod async_search;
pub mod cache;
pub mod config;
pub mod error_reporting;
pub mod export;
pub mod import;
pub mod legacy;
pub mod performance;
pub mod query;
pub mod search;
pub mod state_sync;
pub mod validation;
pub mod virtual_tree;
pub mod watch;
pub mod workspace;
```

---

### 2. services/ - 业务服务层

**位置**: `src-tauri/src/services/mod.rs`

核心业务逻辑实现，采用模块化设计。

| 模块文件 | 功能描述 |
|---------|---------|
| `event_bus.rs` | 事件总线（优先级队列） |
| `file_change_detector.rs` | 文件变化检测 |
| `file_type_filter.rs` | 文件类型过滤 |
| `file_watcher.rs` | 文件监听服务 |
| `index_validator.rs` | 索引验证器 |
| `intelligent_file_filter.rs` | 智能文件过滤 |
| `metadata_db.rs` | 元数据数据库 |
| `pattern_matcher.rs` | Aho-Corasick模式匹配 |
| `query_executor.rs` | 查询执行器 |
| `query_planner.rs` | 查询计划器 |
| `query_validator.rs` | 查询验证器 |
| `regex_engine.rs` | 正则表达式引擎 |
| `report_collector.rs` | 报告收集器 |
| `search_statistics.rs` | 搜索统计服务 |
| `service_config.rs` | 服务配置 |
| `service_container.rs` | 服务容器（DI） |
| `service_lifecycle.rs` | 服务生命周期管理 |
| `traits.rs` | 服务Trait定义 |
| `workspace_metrics.rs` | 工作区指标收集 |

**测试模块**:
- `dependency_management_tests.rs`
- `error_handling_property_tests.rs`
- `concurrency_property_tests.rs`
- `integration_tests.rs`

**关键导出**:
```rust
pub use event_bus::{get_event_bus, AppEvent, EventBus, EventSubscriber};
pub use file_change_detector::{FileChangeDetector, FileChangeStatus};
pub use file_type_filter::FileTypeFilter;
pub use file_watcher::{append_to_workspace_index, get_file_metadata, parse_log_lines, read_file_from_offset};
pub use index_validator::{IndexValidator, InvalidFileInfo, ValidationReport};
pub use intelligent_file_filter::IntelligentFileFilter;
pub use metadata_db::MetadataDB;
pub use query_executor::{MatchDetail, QueryExecutor};
pub use query_planner::{ExecutionPlan, QueryPlannerAdapter};
pub use regex_engine::{AhoCorasickEngine, AutomataEngine, EngineError, EngineInfo, EngineMatches, EngineType, MatchResult, RegexEngine, StandardEngine};
pub use report_collector::{create_archive_error, create_error, create_io_error, create_security_error, ReportCollector};
pub use search_statistics::calculate_keyword_statistics;
pub use service_config::ServiceConfiguration;
pub use service_container::{AppServices, AppServicesBuilder};
pub use service_lifecycle::{HealthStatus, OverallHealth, Service, ServiceHealth, ServiceLifecycleManager};
pub use traits::{ContentStorage, MetadataStorage, PlanResult, QueryExecutor as QueryExecutorTrait, QueryPlanning, QueryValidation, ValidationResult};
pub use workspace_metrics::{DepthDistribution, WorkspaceMetrics, WorkspaceMetricsCollector};
```

---

### 3. search_engine/ - 搜索引擎

**位置**: `src-tauri/src/search_engine/mod.rs`

基于Tantivy的全文搜索引擎实现。

| 模块文件 | 功能描述 |
|---------|---------|
| `advanced_features.rs` | 高级搜索特性 |
| `boolean_query_processor.rs` | 布尔查询处理器 |
| `concurrent_search.rs` | 并发搜索 |
| `disk_result_store.rs` | 磁盘结果存储 |
| `highlighting_engine.rs` | 高亮引擎 |
| `index_optimizer.rs` | 索引优化器 |
| `manager.rs` | 搜索引擎管理器 |
| `query_optimizer.rs` | 查询优化器 |
| `schema.rs` | Tantivy Schema定义 |
| `streaming_builder.rs` | 流式索引构建器 |
| `virtual_search_manager.rs` | 虚拟搜索管理器 |

**测试模块**:
- `property_tests.rs`

**核心类型**:
```rust
pub struct SearchEngineManager {
    // Tantivy索引管理
}

pub struct BooleanQueryProcessor {
    // AND/OR/NOT查询处理
}

pub enum SearchError {
    Timeout(String),
    IndexError(String),
    QueryError(String),
    IoError(std::io::Error),
    TantivyError(tantivy::TantivyError),
    RegexError(regex::Error),
}
```

**关键导出**:
```rust
pub use advanced_features::{AutocompleteEngine, FilterEngine, RegexSearchEngine, TimePartitionedIndex};
pub use boolean_query_processor::BooleanQueryProcessor;
pub use concurrent_search::{ConcurrentSearchConfig, ConcurrentSearchManager, ConcurrentSearchStats};
pub use disk_result_store::{DiskResultStore, SearchPageResult};
pub use highlighting_engine::{HighlightingConfig, HighlightingEngine, HighlightingStats};
pub use manager::SearchEngineManager;
pub use query_optimizer::QueryOptimizer;
pub use schema::LogSchema;
pub use streaming_builder::StreamingIndexBuilder;
pub use virtual_search_manager::{VirtualSearchManager, VirtualSearchStats};
```

---

### 4. storage/ - CAS存储层

**位置**: `src-tauri/src/storage/mod.rs`

内容寻址存储(Content-Addressable Storage)实现。

| 模块文件 | 功能描述 |
|---------|---------|
| `cache_monitor.rs` | 缓存监控 |
| `cas.rs` | CAS核心实现 |
| `coordinator.rs` | 存储协调器 |
| `gc.rs` | 垃圾回收 |
| `integrity.rs` | 完整性校验 |
| `metadata_store.rs` | 元数据存储（SQLite） |
| `metrics_store.rs` | 指标存储 |

**关键特性**:
- SHA-256内容寻址
- 自动去重
- SQLite元数据管理
- FTS5全文搜索

---

### 5. archive/ - 压缩包处理

**位置**: `src-tauri/src/archive/mod.rs`

支持多种压缩格式的统一处理框架。

**核心处理器**:
| 处理器文件 | 支持格式 | 依赖库 |
|-----------|---------|-------|
| `zip_handler.rs` | .zip | zip crate |
| `tar_handler.rs` | .tar, .tar.gz, .tgz | tar + flate2 |
| `gz_handler.rs` | .gz | flate2 |
| `rar_handler.rs` | .rar | unrar (libunrar绑定) |
| `sevenz_handler.rs` | .7z | sevenz-rust |

**完整模块列表** (27个):

| 模块 | 功能描述 |
|------|---------|
| `actors/` | Actor模型处理子模块 |
| `fault_tolerance/` | 容错机制子模块 |
| `streaming/` | 流式处理子模块 |
| `archive_handler.rs` | ArchiveHandler Trait定义 |
| `resource_manager.rs` | 资源管理器 |
| `security_detector.rs` | 安全检测器 |
| `audit_logger.rs` | 审计日志 |
| `checkpoint_manager.rs` | 检查点管理 |
| `compression_analyzer.rs` | 压缩分析器 |
| `edge_case_handlers.rs` | 边缘情况处理 |
| `extraction_context.rs` | 解压上下文 |
| `extraction_engine.rs` | 解压引擎 |
| `extraction_orchestrator.rs` | 解压编排器 |
| `extraction_service.rs` | 解压服务 |
| `gz_handler.rs` | GZ格式处理 |
| `nested_archive_config.rs` | 嵌套压缩配置 |
| `parallel_processor.rs` | 并行处理器 |
| `path_manager.rs` | 路径管理器 |
| `path_validator.rs` | 路径安全验证 |
| `processor.rs` | 递归处理器 |
| `progress_tracker.rs` | 进度追踪器 |
| `public_api.rs` | 公共API |
| `rar_handler.rs` | RAR格式处理 |
| `sevenz_handler.rs` | 7Z格式处理 |
| `tar_handler.rs` | TAR格式处理 |
| `traversal.rs` | 统一遍历模块 |
| `zip_handler.rs` | ZIP格式处理 |

**子模块结构**:
- `actors/` - Actor模型处理
  - `coordinator.rs`
  - `extractor_actor.rs`
  - `progress_tracker.rs`
- `streaming/` - 流式处理
  - `memory_efficient_extractor.rs`
  - `streaming_extractor.rs`
- `fault_tolerance/` - 容错机制
  - `circuit_breaker.rs`
  - `error_recovery.rs`
  - `health_monitor.rs`

**ArchiveManager核心结构**:
```rust
pub struct ArchiveManager {
    handlers: Vec<Box<dyn ArchiveHandler>>,
    max_file_size: u64,    // 单个文件最大大小
    max_total_size: u64,   // 解压后总大小限制
    max_file_count: usize, // 解压文件数量限制
}
```

---

### 6. models/ - 数据模型

**位置**: `src-tauri/src/models/mod.rs`

| 模块文件 | 功能描述 |
|---------|---------|
| `cache_state.rs` | 缓存状态模型 |
| `config.rs` | 配置模型 |
| `extraction_policy.rs` | 解压策略模型 |
| `filters.rs` | 过滤器模型 |
| `import_decision.rs` | 导入决策模型 |
| `log_entry.rs` | 日志条目模型 |
| `metrics_state.rs` | 指标状态模型 |
| `policy_manager.rs` | 策略管理器 |
| `processing_report.rs` | 处理报告模型 |
| `search.rs` | 搜索模型 |
| `search_state.rs` | 搜索状态模型 |
| `search_statistics.rs` | 搜索统计模型 |
| `state.rs` | 应用状态模型 |
| `workspace_state.rs` | 工作区状态模型 |
| `validated.rs` | 验证模型 |

---

### 7. events/ - 事件系统

**位置**: `src-tauri/src/events/mod.rs`

**EventBus架构**:
```rust
pub struct EventBus {
    priority_channels: PriorityEventChannels,
}

pub struct PriorityEventChannels {
    high: (Sender, Receiver),    // 容量: 5000
    normal: (Sender, Receiver),  // 容量: 2000
    low: (Sender, Receiver),     // 容量: 500
}
```

**AppEvent枚举** (18种事件类型):
- `SearchStarted`, `SearchProgress`, `SearchCompleted`
- `TaskCreated`, `TaskProgress`, `TaskCompleted`
- `WorkspaceCreated`, `WorkspaceUpdated`, `WorkspaceDeleted`
- `FileImported`, `FileProcessed`, `FileError`
- 等等...

---

### 8. task_manager/ - 任务管理器

**位置**: `src-tauri/src/task_manager/mod.rs`

基于Actor模型的任务管理系统。

**核心类型**:
```rust
pub struct TaskManager {
    sender: mpsc::UnboundedSender<ActorMessage>,
    config: TaskManagerConfig,
}

pub struct TaskInfo {
    pub task_id: String,
    pub task_type: String,
    pub target: String,
    pub progress: u8,
    pub message: String,
    pub status: TaskStatus,
    pub version: u64,  // 幂等性检查
    pub workspace_id: Option<String>,
}

pub enum TaskStatus {
    Running,
    Completed,
    Failed,
    Stopped,
}

pub struct TaskManagerMetrics {
    pub total_tasks: usize,
    pub running_tasks: usize,
    pub completed_tasks: usize,
    pub failed_tasks: usize,
    pub stopped_tasks: usize,
    pub is_healthy: bool,
}
```

**Actor消息类型**:
- `CreateTask`, `UpdateTask`, `GetTask`
- `GetAllTasks`, `RemoveTask`, `GetMetrics`
- `CleanupExpired`, `Shutdown`

---

### 9. domain/ - 领域层（DDD）

**位置**: `src-tauri/src/domain/mod.rs`

采用领域驱动设计模式。

**子模块**:
- `log_analysis/` - 日志分析领域
  - `entities.rs` - 实体（LogEntry）
  - `value_objects.rs` - 值对象（LogLevel, LogMessage, Timestamp）
- `shared/` - 共享领域组件
  - `events.rs` - 领域事件总线

**关键导出**:
```rust
pub use shared::events::DomainEventBus;
pub use log_analysis::{LogEntry, LogLevel, LogMessage, Timestamp};
```

---

### 10. application/ - 应用层（DDD）

**位置**: `src-tauri/src/application/mod.rs`

协调领域层用例的应用服务。

**子模块**:
- `commands/` - 应用命令
- `plugins/` - 插件系统
  - `Plugin` trait定义
  - `PluginManager` 动态库管理
  - ABI版本验证（v1.0）
  - 目录白名单安全机制
- `services/` - 应用服务

---

### 11. infrastructure/ - 基础设施层

**位置**: `src-tauri/src/infrastructure/mod.rs`

**子模块**:
- `config/` - 配置管理

---

### 12. monitoring/ - 监控和可观测性

**位置**: `src-tauri/src/monitoring/mod.rs`

**核心组件**:
```rust
pub struct MetricsCollector {
    search_counter: Arc<metrics::Counter>,
    error_counter: Arc<metrics::Counter>,
    search_duration: Arc<metrics::Histogram>,
}

pub struct HealthCheck {
    pub service_name: String,
    pub status: HealthStatus,
    pub details: Option<String>,
    pub timestamp: SystemTime,
}

pub enum HealthStatus {
    Healthy,
    Degraded,
    Unhealthy,
}
```

**子模块**:
- `metrics.rs` - 指标收集

---

### 13. state_sync/ - 状态同步

**位置**: `src-tauri/src/state_sync/mod.rs`

基于Tauri Events的实时状态同步（<10ms延迟）。

```rust
pub struct StateSync {
    app_handle: AppHandle,
    state_cache: Arc<RwLock<HashMap<String, WorkspaceState>>>,
    event_history: Arc<RwLock<VecDeque<WorkspaceEvent>>>,
    max_history_size: usize,  // 默认1000
}
```

**WorkspaceEvent类型**:
- `StatusChanged`, `ProgressUpdate`, `TaskCompleted`, `Error`

---

### 14. utils/ - 工具模块

**位置**: `src-tauri/src/utils/mod.rs`

| 模块文件 | 功能描述 |
|---------|---------|
| `async_resource_manager.rs` | 异步资源管理 |
| `cache_manager.rs` | 缓存管理 |
| `cancellation_manager.rs` | 取消令牌管理 |
| `cleanup.rs` | 资源清理 |
| `encoding.rs` | 多编码支持（UTF-8/GBK/Windows-1252） |
| `legacy_detection.rs` | 遗留检测 |
| `path.rs` | 路径处理（Windows UNC支持） |
| `path_security.rs` | 路径安全检查 |
| `resource_manager.rs` | 资源管理器 |
| `resource_tracker.rs` | 资源追踪器 |
| `retry.rs` | 重试机制（指数退避） |
| `validation.rs` | 输入验证 |

---

### 15. error.rs - 错误处理

**位置**: `src-tauri/src/error.rs`

使用`thiserror`定义的统一错误类型。

**AppError枚举** (20+错误变体):
```rust
pub enum AppError {
    Io { source: std::io::Error, context: String },
    Archive { message: String, path: Option<PathBuf> },
    Search { message: String },
    Database { message: String },
    Validation { field: String, message: String },
    NotFound { resource: String },
    // ... 更多错误类型
}
```

**CommandError** - Tauri命令响应包装

---

## React前端模块

### 顶层目录结构

```
log-analyzer/src/
├── components/       # UI组件
├── constants/        # 常量定义
├── hooks/            # 自定义Hooks（22个）
├── i18n/             # 国际化
├── pages/            # 页面组件
├── providers/        # React Query Provider
├── services/         # API服务
├── stores/           # Zustand状态管理
├── test-utils/       # 测试工具
├── types/            # TypeScript类型
├── App.tsx           # 主应用组件
└── main.tsx          # 入口文件
```

---

### 1. stores/ - 状态管理

**位置**: `log-analyzer/src/stores/`

使用Zustand进行全局状态管理。

| 文件 | 功能描述 |
|-----|---------|
| `appStore.ts` | 应用全局状态 |
| `workspaceStore.ts` | 工作区状态 |
| `keywordStore.ts` | 关键词状态 |
| `taskStore.ts` | 任务状态 |
| `types.ts` | Store类型定义 |
| `index.ts` | 导出入口 |
| `AppStoreProvider.tsx` | Provider组件 |

---

### 2. hooks/ - 自定义Hooks

**位置**: `log-analyzer/src/hooks/`

| Hook文件 | 功能描述 |
|---------|---------|
| `useConfig.ts` | 配置管理 |
| `useConfigManager.ts` | 配置管理器 |
| `useFormValidation.ts` | 表单验证 |
| `useInfiniteSearch.ts` | 无限滚动搜索 |
| `useKeyboardShortcuts.ts` | 键盘快捷键 |
| `useKeywordManager.ts` | 关键词管理 |
| `usePerformanceQueries.ts` | 性能查询 |
| `useSearchListeners.ts` | 搜索事件监听 |
| `useServerQueries.ts` | 服务端查询 |
| `useTaskManager.ts` | 任务管理 |
| `useToast.ts` | Toast消息 |
| `useToastManager.ts` | Toast管理器 |
| `useWorkspaceImport.ts` | 工作区导入 |
| `useWorkspaceList.ts` | 工作区列表 |
| `useWorkspaceManagement.ts` | 工作区管理 |
| `useWorkspaceMutations.ts` | 工作区变更 |
| `useWorkspaceOperations.ts` | 工作区操作 |
| `useWorkspaceSelection.ts` | 工作区选择 |
| `useWorkspaceWatch.ts` | 工作区监听 |
| `useResourceManager.ts` | 资源管理 |
| `useQueryErrorHandler.ts` | 查询错误处理 |
| `useErrorManagement.ts` | 错误管理 |
| `index.ts` | 导出入口 |

---

### 3. services/ - API服务

**位置**: `log-analyzer/src/services/`

| 文件 | 功能描述 |
|-----|---------|
| `api.ts` | Tauri IPC封装 |
| `queryApi.ts` | 查询API |
| `fileApi.ts` | 文件API |
| `SearchQueryBuilder.ts` | 查询构建器（流畅API） |
| `queryStorage.ts` | 查询持久化（localStorage） |
| `errors.ts` | 错误处理服务 |
| `nullSafeApi.ts` | 空安全API封装 |

---

### 4. components/ - UI组件

**位置**: `log-analyzer/src/components/`

#### 4.1 ui/ - 基础UI组件
| 文件 | 功能描述 |
|-----|---------|
| `Button.tsx` | 按钮组件 |
| `Card.tsx` | 卡片组件 |
| `FormField.tsx` | 表单字段 |
| `Input.tsx` | 输入框 |
| `NavItem.tsx` | 导航项 |

#### 4.2 modals/ - 模态框
| 文件 | 功能描述 |
|-----|---------|
| `FilterPalette.tsx` | 过滤器面板 |
| `KeywordModal.tsx` | 关键词管理模态框 |
| `FileFilterSettings.tsx` | 文件过滤设置 |

#### 4.3 renderers/ - 渲染器
| 文件 | 功能描述 |
|-----|---------|
| `HybridLogRenderer.tsx` | 混合日志渲染器（虚拟滚动） |

#### 4.4 search/ - 搜索组件
| 文件 | 功能描述 |
|-----|---------|
| `KeywordStatsPanel.tsx` | 关键词统计面板 |

#### 4.5 charts/ - 图表组件
| 文件 | 功能描述 |
|-----|---------|
| `MetricsTimeSeriesChart.tsx` | 指标时间序列图 |
| `TimeRangeSelector.tsx` | 时间范围选择器 |

#### 4.6 其他组件
| 文件 | 功能描述 |
|-----|---------|
| `ErrorBoundary.tsx` | 错误边界 |
| `EventManager.tsx` | 事件管理器 |
| `VirtualFileTree.tsx` | 虚拟文件树 |

---

### 5. pages/ - 页面组件

**位置**: `log-analyzer/src/pages/`

| 文件/目录 | 功能描述 |
|----------|---------|
| `SearchPage.tsx` | 搜索页面（主页面） |
| `SearchPage/components/` | 搜索页面子组件 |
| ├── `ActiveKeywords.tsx` | 活动关键词 |
| ├── `SearchControls.tsx` | 搜索控件 |
| ├── `SearchFilters.tsx` | 搜索过滤器 |
| `KeywordsPage.tsx` | 关键词管理页面 |
| `WorkspacesPage.tsx` | 工作区管理页面 |
| `TasksPage.tsx` | 任务管理页面 |
| `PerformancePage.tsx` | 性能监控页面 |
| `SettingsPage.tsx` | 设置页面 |

---

### 6. types/ - TypeScript类型

**位置**: `log-analyzer/src/types/`

| 文件 | 功能描述 |
|-----|---------|
| `search.ts` | 搜索相关类型 |
| `common.ts` | 通用类型（Workspace, LogEntry等） |
| `ui.ts` | UI相关类型 |

---

### 7. constants/ - 常量定义

**位置**: `log-analyzer/src/constants/`

- 颜色常量
- 搜索配置常量

---

### 8. i18n/ - 国际化

**位置**: `log-analyzer/src/i18n/`

- `locales/zh.json` - 中文翻译
- `locales/en.json` - 英文翻译

---

### 9. providers/ - Provider组件

**位置**: `log-analyzer/src/providers/`

| 文件 | 功能描述 |
|-----|---------|
| `QueryProvider.tsx` | React Query Provider |

---

## 模块依赖关系

### 后端模块依赖图

```
main.rs
   │
   └── lib.rs
        ├── commands/ ←→ services/
        │      │            │
        │      └────────────┘
        │
        ├── services/
        │      ├── search_engine/
        │      ├── storage/
        │      ├── events/
        │      └── utils/
        │
        ├── task_manager/ ←→ events/
        │
        ├── archive/ ←→ storage/
        │
        ├── domain/
        │      └── application/ ←→ infrastructure/
        │
        ├── state_sync/ ←→ events/
        │
        └── monitoring/
```

### 前端模块依赖图

```
App.tsx
   │
   ├── providers/QueryProvider
   │
   ├── stores/ (Zustand)
   │      ├── appStore
   │      ├── workspaceStore
   │      ├── keywordStore
   │      └── taskStore
   │
   ├── pages/
   │      ├── SearchPage
   │      ├── WorkspacesPage
   │      ├── KeywordsPage
   │      ├── TasksPage
   │      └── PerformancePage
   │
   ├── hooks/ ←→ services/api
   │
   └── components/
          ├── ui/
          ├── modals/
          ├── renderers/
          └── search/
```

---

## 架构分层

### 后端分层架构（DDD）

```
┌─────────────────────────────────────────────────┐
│                  commands/                       │
│              (Tauri IPC 接口层)                   │
├─────────────────────────────────────────────────┤
│                 application/                     │
│           (应用服务 + 插件系统)                    │
├─────────────────────────────────────────────────┤
│                   domain/                        │
│        (领域实体 + 值对象 + 领域事件)               │
├─────────────────────────────────────────────────┤
│               infrastructure/                    │
│           (配置 + 外部服务集成)                    │
├─────────────────────────────────────────────────┤
│    services/ │ search_engine/ │ storage/         │
│         (核心业务服务层)                          │
├─────────────────────────────────────────────────┤
│    archive/ │ task_manager/ │ events/            │
│         (基础设施服务)                            │
└─────────────────────────────────────────────────┘
```

### 前端分层架构

```
┌─────────────────────────────────────────────────┐
│                   pages/                         │
│                (页面组件)                         │
├─────────────────────────────────────────────────┤
│                components/                       │
│              (UI组件 + 渲染器)                    │
├─────────────────────────────────────────────────┤
│                   hooks/                         │
│           (业务逻辑 + 状态管理)                    │
├─────────────────────────────────────────────────┤
│          stores/ │ services/                     │
│        (状态管理 + API封装)                       │
└─────────────────────────────────────────────────┘
```

---

## 统计信息

### Rust后端

| 类别 | 数量 |
|-----|------|
| 顶层模块 | 15个 |
| 命令模块 | 15个 |
| 服务模块 | 19个 |
| 搜索模块 | 11个 |
| 存储模块 | 7个 |
| 压缩模块 | 27个 |
| 模型模块 | 15个 |
| 工具模块 | 12个 |

### React前端

| 类别 | 数量 |
|-----|------|
| 页面组件 | 6个 |
| Hooks | 22个 |
| Store | 7个 |
| 服务模块 | 7个 |
| UI组件 | 15+个 |

---

*本文档基于代码分析自动生成，最后更新: 2026-03-28*

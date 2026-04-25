# IPC API 概览

前端通过 Tauri `invoke()` 调用后端命令，通过 `emit/listen` 接收后端事件。

**命令注册入口：** `log-analyzer/src-tauri/src/main.rs`（`generate_handler!` 宏）

**命令实现目录：** `log-analyzer/src-tauri/src/commands/`

**前端 API 封装：** `log-analyzer/src/services/api.ts`

---

## 搜索命令

### `search_logs`

**用途：** 启动一次搜索，返回 `search_id`，后续通过 `fetch_search_page` 分页拉取结果。

**参数：**

| 参数 | 类型 | 说明 |
|------|------|------|
| `workspace_id` | `string` | 工作区 ID |
| `query` | `string` | 查询字符串（`\|` 分隔多关键词，OR 逻辑） |
| `filters` | `SearchFilters \| null` | 可选过滤条件 |

**`SearchFilters` 结构：**

```typescript
interface SearchFilters {
  log_levels?: string[];        // 日志级别白名单（大小写不敏感）
  time_range?: {
    start?: string;             // ISO8601 或 datetime-local 格式
    end?: string;
  };
  file_pattern?: string;        // 文件路径模式（支持 * ? 通配符）
}
```

**返回：**

```typescript
interface SearchStartResult {
  search_id: string;
  total_count: number;
}
```

**当前真实行为：**

- 查询以 `|` 分割为多个 `SearchTerm`（OR 逻辑）
- 多关键词时启用 Aho-Corasick 快速预检引擎
- 过滤条件在搜索开始前预编译（一次性）
- 结果写入磁盘临时文件，分页读取
- 工作区运行态在内存丢失时自动从磁盘懒恢复

---

### `fetch_search_page`

**用途：** 按 `search_id + offset + limit` 分页读取搜索结果。

**参数：**

| 参数 | 类型 | 说明 |
|------|------|------|
| `search_id` | `string` | `search_logs` 返回的会话 ID |
| `offset` | `number` | 结果偏移（0-based） |
| `limit` | `number` | 每页条数（推荐 50） |

**返回：**

```typescript
interface SearchPageResult {
  entries: LogEntry[];
  has_more: boolean;
}

interface LogEntry {
  id: number;                           // 结果集内唯一顺序 ID
  timestamp: string;                    // 原始时间戳字符串
  level: string;                        // 日志级别
  file: string;                         // 虚拟文件路径
  real_path: string;                    // CAS 路径或原始路径
  line: number;                         // 行号（1-based）
  content: string;                      // 原始行内容
  match_details?: MatchDetail[];        // 命中位置信息（字节偏移）
  matched_keywords?: string[];          // 命中的关键词
}

interface MatchDetail {
  keyword: string;
  start: number;   // 字节偏移
  end: number;
}
```

**读取优先级：**

1. 优先从 `DiskResultStore` 磁盘会话读取
2. 无磁盘会话时回退到 `VirtualSearchManager` 内存会话（兼容旧路径）

---

### `cancel_search`

**用途：** 取消正在执行的搜索任务。

**参数：** `{ search_id: string }`

**机制：** 通过 `CancellationToken` 通知搜索任务提前终止，已写入磁盘的结果保留。

---

## 结构化查询命令

### `execute_structured_query`

**用途：** 执行结构化查询（非 UI 主搜索路径，用于测试/高级调用）。

> **注意：** 当前 UI 主搜索不走此命令。主搜索入口是 `search_logs`。

### `validate_query`

**用途：** 在执行前校验结构化查询合法性（正则语法、长度限制等）。

---

## 搜索会话管理

| 命令 | 用途 |
|------|------|
| `register_search_session` | 注册新搜索会话（含查询元信息） |
| `get_search_session_info` | 获取会话元信息（query、result_count、状态） |
| `get_search_total_count` | 获取指定会话的结果总数 |
| `remove_search_session` | 手动移除搜索会话（释放磁盘临时文件） |
| `cleanup_expired_search_sessions` | 清理过期会话（默认 30 分钟过期） |
| `get_virtual_search_stats` | 获取搜索引擎运行统计（命中率、平均耗时等） |

---

## 异步搜索

| 命令 | 用途 |
|------|------|
| `async_search_logs` | 通过 `TaskManager` 调度后台搜索任务 |
| `cancel_async_search` | 取消指定的异步搜索任务 |
| `get_active_searches_count` | 查询当前活跃搜索任务数 |

与 `search_logs` 的区别：`async_search_logs` 立即返回 task_id，搜索在后台运行，完成后通过事件通知前端。适合不阻塞 UI 的长时间搜索。

---

## 工作区命令

### 核心工作区操作

| 命令 | 参数 | 用途 |
|------|------|------|
| `create_workspace` | `name, path` | 创建新工作区，初始化 CAS 和元数据库 |
| `load_workspace` | `workspace_id` | 加载工作区，恢复运行态（CAS、MetadataStore、SearchEngineManager） |
| `refresh_workspace` | `workspace_id` | 重新扫描工作区目录，更新元数据 |
| `delete_workspace` | `workspace_id` | 删除工作区（磁盘数据 + 内存运行态） |
| `get_workspace_time_range` | `workspace_id` | 获取工作区日志的时间跨度（min/max 时间戳） |
| `get_workspace_status` | `workspace_id` | 获取工作区状态（文件数、大小、导入状态） |
| `cancel_task` | `task_id` | 取消工作区相关后台任务 |

**工作区路径约定：**

```text
{app_data_dir}/workspaces/{workspace_id}/
  objects/      ← CAS 对象目录
  metadata.db   ← SQLite 元数据库
```

### 导入命令

| 命令 | 用途 |
|------|------|
| `import_folder` | 导入文件夹（含压缩包递归解压） |
| `check_rar_support` | 检测当前构建是否编译了 RAR 支持，以及当前运行时是否可用 |

**`import_folder` 事件流（通过 Tauri emit）：**

```text
import-start    → { workspace_id, total_files }
import-progress → { workspace_id, processed, total, current_file }
import-complete → { workspace_id, total_files, total_size }
import-error    → { workspace_id, error }
```

---

## 文件监听与虚拟文件树

| 命令 | 用途 |
|------|------|
| `start_watch` | 启动文件监听（inotify/FSEvents/ReadDirectoryChangesW） |
| `stop_watch` | 停止文件监听 |
| `get_virtual_file_tree` | 获取工作区虚拟文件树（从元数据重建） |
| `read_file_by_hash` | 通过 SHA-256 读取 CAS 文件内容 |

**虚拟文件树节点结构：**

```typescript
interface TreeNode {
  name: string;
  virtual_path: string;
  node_type: 'file' | 'directory' | 'archive';
  size?: number;
  depth_level: number;
  children?: TreeNode[];
  sha256_hash?: string;  // 仅文件节点有
}
```

---

## 配置管理

| 命令 | 用途 |
|------|------|
| `load_config` / `save_config` | 应用全局配置（含所有子配置）读写 |
| `get_cache_config` / `save_cache_config` | 缓存策略（容量、TTL、TTI、压缩阈值） |
| `get_search_config` / `save_search_config` | 搜索引擎配置（最大结果数、超时） |
| `get_file_filter_config` / `save_file_filter_config` | 文件过滤规则（白名单/黑名单） |
| `get_task_manager_config` / `save_task_manager_config` | 任务管理器（并发度、超时） |

**配置存储路径：** `{config_dir}/config.json`

**`CacheConfig` 关键字段：**

```typescript
interface CacheConfig {
  max_capacity: number;         // 最大缓存条目数
  ttl_seconds: number;          // 过期时间（秒，0=永不过期）
  tti_seconds: number;          // 空闲过期时间（秒，0=不启用）
  compression_threshold: number; // 启用压缩的大小阈值（字节）
}
```

---

## 日志配置管理

| 命令 | 用途 |
|------|------|
| `get_current_log_config` | 获取当前日志配置（模块级别映射） |
| `set_log_level` | 设置全局日志级别（TRACE/DEBUG/INFO/WARN/ERROR） |
| `set_module_level` | 设置指定模块的日志级别（运行时生效） |
| `reset_log_configuration` | 重置为默认配置 |
| `get_recommended_production_config` | 获取生产环境推荐配置 |
| `get_recommended_debug_config` | 获取调试环境推荐配置 |
| `load_log_config` / `save_log_config` | 持久化日志配置 |
| `get_available_log_levels` | 枚举可用级别列表 |
| `apply_log_preset` | 应用预设方案（production/debug/quiet） |

---

## 状态同步

| 命令 | 用途 |
|------|------|
| `init_state_sync` | 初始化状态同步通道（建立 Tauri 事件监听） |
| `get_workspace_state` | 获取指定工作区的当前实时状态 |
| `get_event_history` | 查询最近 N 个状态事件（前端重连后回放） |
| `broadcast_test_event` | 广播测试事件（调试用，不用于生产） |

---

## 数据验证

| 命令 | 用途 |
|------|------|
| `validate_workspace_config_cmd` | 校验工作区配置合法性 |
| `validate_search_query_cmd` | 校验搜索查询（正则语法、长度等） |
| `validate_archive_config_cmd` | 校验归档解压配置 |
| `batch_validate_workspace_configs` | 批量校验多个工作区配置 |
| `validate_workspace_id_format` | 校验工作区 ID 格式（只含字母数字下划线） |
| `validate_path_security` | 校验路径不含路径穿越攻击字符（`../`） |

---

## 导出与缓存

| 命令 | 用途 |
|------|------|
| `export_results` | 导出搜索结果（CSV/JSON/TXT） |
| `invalidate_workspace_cache` | 手动使指定工作区的 L1 缓存失效 |

---

## 错误报告

| 命令 | 用途 |
|------|------|
| `report_frontend_error` | 前端运行时异常上报（记录到 SQLite） |
| `submit_user_feedback` | 用户反馈提交 |
| `get_error_statistics` | 错误统计查询（按类型、时间段） |

---

## 传统格式兼容

| 命令 | 用途 |
|------|------|
| `scan_legacy_formats` | 扫描并识别旧版本工作区格式 |
| `get_legacy_workspace_info` | 获取旧格式工作区的基本信息 |

---

## 后端推送事件

前端通过 `listen(eventName, handler)` 订阅。

### 搜索事件

| 事件名 | 数据 | 说明 |
|--------|------|------|
| `search-start` | `{ search_id }` | 搜索开始 |
| `search-progress` | `{ search_id, scanned_files, matched_lines }` | 搜索进度 |
| `search-summary` | `{ search_id, total_count, duration_ms }` | 搜索完成摘要 |
| `search-complete` | `{ search_id }` | 搜索完全结束 |
| `search-cancelled` | `{ search_id }` | 搜索已取消 |
| `search-timeout` | `{ search_id }` | 搜索超时 |
| `search-error` | `{ search_id, error }` | 搜索出错 |

### 导入事件

| 事件名 | 数据 | 说明 |
|--------|------|------|
| `import-start` | `{ workspace_id, total_files }` | 导入开始 |
| `import-progress` | `{ workspace_id, processed, total, current_file }` | 导入进度 |
| `import-complete` | `{ workspace_id, total_files, total_size }` | 导入完成 |
| `import-error` | `{ workspace_id, error }` | 导入出错 |

### 文件监听事件

| 事件名 | 数据 | 说明 |
|--------|------|------|
| `file-change` | `{ workspace_id, path, change_type }` | 文件变化（创建/修改/删除） |

### 工作区事件

| 事件名 | 数据 | 说明 |
|--------|------|------|
| `workspace-updated` | `{ workspace_id }` | 工作区状态变化 |

---

## 集成注意事项

- 修改命令参数或返回值时，必须同步更新：`src/services/api.ts`、对应页面组件、本文档
- 前端 `entry.id` 用于虚拟滚动和选中状态，后端保证同一 `search_id` 会话内唯一
- 时间过滤值支持 `datetime-local` 格式（`YYYY-MM-DDTHH:MM`）和 RFC3339；无法解析时视为未设置
- 文件模式（`file_pattern`）含通配符时按 glob 解析，否则按子串匹配
- `search_id` 默认 30 分钟过期，前端应在过期前完成分页拉取

---

## 相关代码

- 命令注册：`log-analyzer/src-tauri/src/main.rs`
- 搜索实现：`log-analyzer/src-tauri/src/commands/search.rs`
- API 封装：`log-analyzer/src/services/api.ts`
- 模块架构：[MODULE_ARCHITECTURE.md](./modules/MODULE_ARCHITECTURE.md)

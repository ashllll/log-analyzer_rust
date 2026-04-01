# IPC API 概览

本文件只记录当前仍在主链路中使用、且对前后端集成有意义的接口。

## 调用方式

前端通过 Tauri `invoke()` 调用命令，通过 `emit/listen` 接收事件。

命令注册入口：

- `log-analyzer/src-tauri/src/main.rs`

命令实现目录：

- `log-analyzer/src-tauri/src/commands/`

前端 API 包装：

- `log-analyzer/src/services/api.ts`

## 搜索相关命令

### `search_logs`

用途：

- 启动一次搜索
- 返回 `search_id`
- 后续通过 `fetch_search_page` 拉取结果页

当前真实行为：

- 查询字符串以 `|` 分割为多个关键词
- 多关键词使用 OR 逻辑
- 过滤器支持级别、时间范围、文件模式
- 结果先写入后端磁盘会话，再分页读取

### `fetch_search_page`

用途：

- 按 `search_id + offset + limit` 获取结果页

读取顺序：

1. 优先从磁盘结果存储读取
2. 无磁盘会话时回退到旧的内存会话管理器

### `cancel_search`

用途：

- 取消正在执行的搜索任务

## 结构化查询命令

### `execute_structured_query`

用途：

- 用于结构化查询能力验证或独立调用

注意：

- 当前 UI 主搜索并不直接走这条命令
- 不应把它误当成当前搜索主链路

### `validate_query`

用途：

- 校验结构化查询对象

## 流式搜索会话管理

<!-- AUTO-GENERATED: start -->
| 命令 | 用途 |
|------|------|
| `fetch_search_page` | 按 search_id + offset + limit 分页读取搜索结果 |
| `register_search_session` | 注册新搜索会话 |
| `get_search_session_info` | 获取会话元信息 |
| `get_search_total_count` | 获取搜索结果总数 |
| `remove_search_session` | 手动移除搜索会话 |
| `cleanup_expired_search_sessions` | 清理过期会话 |
| `get_virtual_search_stats` | 获取搜索引擎运行统计 |
<!-- AUTO-GENERATED: end -->

## 异步搜索

<!-- AUTO-GENERATED: start -->
| 命令 | 用途 |
|------|------|
| `async_search_logs` | 异步启动搜索任务（由 TaskManager 调度） |
| `cancel_async_search` | 取消异步搜索 |
| `get_active_searches_count` | 查询当前活跃搜索数 |
<!-- AUTO-GENERATED: end -->

## 工作区与导入命令

核心命令：

- `create_workspace`
- `load_workspace`
- `refresh_workspace`
- `delete_workspace`
- `import_folder`
- `get_workspace_time_range`
- `get_workspace_status`
- `cancel_task`（取消工作区任务）
- `check_rar_support`（检测 RAR 解压支持）

这些命令与 CAS / 元数据存储、虚拟文件树和搜索入口共同组成工作区主流程。

## 文件监听与虚拟文件树

- `start_watch` / `stop_watch`
- `get_virtual_file_tree` / `read_file_by_hash`

## 配置管理

<!-- AUTO-GENERATED: start -->
| 命令 | 用途 |
|------|------|
| `load_config` / `save_config` | 应用全局配置读写 |
| `get_file_filter_config` / `save_file_filter_config` | 文件过滤规则 |
| `get_cache_config` / `save_cache_config` | 缓存策略配置 |
| `get_search_config` / `save_search_config` | 搜索引擎配置 |
| `get_task_manager_config` / `save_task_manager_config` | 任务管理器配置 |
<!-- AUTO-GENERATED: end -->

## 日志配置管理

<!-- AUTO-GENERATED: start -->
| 命令 | 用途 |
|------|------|
| `get_current_log_config` | 获取当前日志配置 |
| `set_log_level` / `set_module_level` | 运行时调整日志级别 |
| `reset_log_configuration` | 重置为默认配置 |
| `get_recommended_production_config` / `get_recommended_debug_config` | 预设配置模板 |
| `load_log_config` / `save_log_config` | 持久化日志配置 |
| `get_available_log_levels` | 枚举可用级别 |
| `apply_log_preset` | 应用预设方案 |
<!-- AUTO-GENERATED: end -->

## 错误报告

<!-- AUTO-GENERATED: start -->
| 命令 | 用途 |
|------|------|
| `report_frontend_error` | 前端异常上报 |
| `submit_user_feedback` | 用户反馈提交 |
| `get_error_statistics` | 错误统计查询 |
<!-- AUTO-GENERATED: end -->

## 状态同步

<!-- AUTO-GENERATED: start -->
| 命令 | 用途 |
|------|------|
| `init_state_sync` | 初始化状态同步通道 |
| `get_workspace_state` | 获取工作区实时状态 |
| `get_event_history` | 查询事件历史 |
| `broadcast_test_event` | 广播测试事件（调试用） |
<!-- AUTO-GENERATED: end -->

## 数据验证

<!-- AUTO-GENERATED: start -->
| 命令 | 用途 |
|------|------|
| `validate_workspace_config_cmd` | 校验工作区配置 |
| `validate_search_query_cmd` | 校验搜索查询 |
| `validate_archive_config_cmd` | 校验归档配置 |
| `batch_validate_workspace_configs` | 批量校验工作区配置 |
| `validate_workspace_id_format` | 校验 ID 格式 |
| `validate_path_security` | 校验路径安全性 |
<!-- AUTO-GENERATED: end -->

## 导出与缓存

- 导出：`export_results`
- 缓存：`invalidate_workspace_cache`

## 性能监控

<!-- AUTO-GENERATED: start -->
| 命令 | 用途 |
|------|------|
| `get_performance_metrics` | 获取当前性能指标 |
| `get_historical_metrics` | 查询历史指标 |
| `get_aggregated_metrics` | 获取聚合指标 |
| `get_search_events` | 搜索事件流 |
| `get_metrics_stats` | 指标统计摘要 |
| `cleanup_metrics_data` | 清理过期指标数据 |
<!-- AUTO-GENERATED: end -->

## 传统格式兼容

- `scan_legacy_formats`
- `get_legacy_workspace_info`

## 搜索事件

搜索链路中前端需要关注的事件：

- `search-start`
- `search-progress`
- `search-summary`
- `search-complete`
- `search-cancelled`
- `search-timeout`
- `search-error`

## 集成注意事项

- 文档、前端与后端必须以当前真实命令行为准
- 若修改搜索命令参数或返回值，必须同步更新：
  - `src/services/api.ts`
  - `src/pages/SearchPage.tsx`
  - 本文档

## 相关代码

- 命令注册：[main.rs](/Users/llll/code/github/log-analyzer_rust/log-analyzer/src-tauri/src/main.rs)
- 搜索实现：[search.rs](/Users/llll/code/github/log-analyzer_rust/log-analyzer/src-tauri/src/commands/search.rs)
- API 封装：[api.ts](/Users/llll/code/github/log-analyzer_rust/log-analyzer/src/services/api.ts)

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

## 工作区与导入命令

核心命令：

- `create_workspace`
- `load_workspace`
- `refresh_workspace`
- `delete_workspace`
- `import_folder`
- `get_workspace_time_range`

这些命令与 CAS / 元数据存储、虚拟文件树和搜索入口共同组成工作区主流程。

## 其他核心命令

- 文件监听：`start_watch` / `stop_watch`
- 虚拟文件树：`get_virtual_file_tree` / `read_file_by_hash`
- 导出：`export_results`
- 性能监控：`get_performance_metrics` 等
- 配置：`load_config` / `save_config` 及若干配置子命令

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

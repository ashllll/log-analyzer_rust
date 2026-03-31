<!-- Parent: ../AGENTS.md -->
<!-- Generated: 2026-03-31 | Updated: 2026-03-31 -->

# commands (Tauri命令)

## Purpose
Tauri IPC命令处理器，暴露给前端调用的所有后端API。

## Key Files

| File | Description |
|------|-------------|
| `mod.rs` | 命令模块导出和注册 |
| `search.rs` | 日志搜索命令 |
| `import.rs` | 文件导入命令 |
| `workspace.rs` | 工作区管理命令 |
| `query.rs` | 结构化查询命令 |
| `export.rs` | 结果导出命令 |
| `watch.rs` | 文件监听命令 |
| `config.rs` | 配置管理命令 |
| `performance.rs` | 性能指标命令 |
| `log_config.rs` | 日志配置命令 |

## For AI Agents

### Working In This Directory
- 每个命令使用 `#[tauri::command]` 宏
- 返回 `CommandResult<T>` 类型
- 错误使用 `AppError` 转换为字符串

### Testing Requirements
- 命令通过集成测试验证
- 使用 `tauri::test` 框架

### Common Patterns
- 使用 `State<AppState>` 访问应用状态
- 使用 `AppHandle` 发送事件到前端
- 异步命令返回 `Result<T, String>`

## Dependencies

### Internal
- `services/` - 业务逻辑
- `models/` - 数据模型
- `utils/` - 工具函数

### External
- **tauri** - 命令宏和类型

<!-- MANUAL: -->

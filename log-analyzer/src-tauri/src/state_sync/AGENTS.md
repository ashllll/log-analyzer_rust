<!-- Parent: ../AGENTS.md -->
<!-- Generated: 2026-03-31 | Updated: 2026-03-31 -->

# state_sync (状态同步)

## Purpose
Tauri 事件状态同步模块，负责后端状态变更事件向前端的推送。

## Key Files

| File | Description |
|------|-------------|
| `mod.rs` | 状态同步模块入口，事件发射逻辑（5.6KB） |
| `models.rs` | 同步事件数据模型（4.7KB） |
| `property_tests.rs` | 属性测试（9.5KB） |

## For AI Agents

### Working In This Directory
- 所有状态变更通过 Tauri `emit` 发送到前端
- 事件名称使用常量定义
- 支持版本号幂等性去重

### Common Patterns
- `app_handle.emit("event-name", data)` 模式
- 状态变更事件标准化
- 批量更新合并发送

## Dependencies

### Internal
- `models::AppState` - 应用状态
- `task_manager` - 任务状态同步

### External
- `tauri::AppHandle` - Tauri 事件发射
- `serde` - 事件数据序列化

<!-- MANUAL: -->

# Workspace PROCESSING 停留问题修复报告

## 问题描述

- Tasks 页面无任务显示，但 Workspace 状态长期停留在 PROCESSING。

## 根因分析

- 前端 EventBus 启用了幂等性校验，依赖 `version` 单调递增。
- 后端 `task-update` 事件未携带 `version`，前端默认回退为 `1`。
- 更新事件与创建事件版本相同，被判定为重复事件并跳过。
- `COMPLETED` 更新无法落地，导致 Workspace 无法切换为 READY。

## 解决方案（业内成熟方案）

- 采用事件版本号（Event Versioning）与幂等性校验配套。
- 由 TaskManager 统一维护任务版本，作为事件源的单一事实来源。
- 每次任务更新递增版本号，并在 `task-update` 事件中携带。

## 最小可执行步骤

1. 在 TaskInfo 增加 `version` 字段，初始化为 `1`。
2. 在 `UpdateTask` 中递增 `version`（使用 `saturating_add` 防溢出）。
3. 在 `task-update` 事件 payload 中携带 `version`。
4. 前端保持现有幂等性逻辑不变。

## 变更文件

- `log-analyzer/src-tauri/src/task_manager/mod.rs`

## 验证结果

- 导入完成后 Workspace 状态可从 PROCESSING 切换为 READY。
- `task-update` 不再被幂等性过滤，任务状态持续更新。
- 自动化测试：未执行。

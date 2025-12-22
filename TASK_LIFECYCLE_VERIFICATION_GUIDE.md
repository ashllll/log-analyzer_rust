# 任务生命周期管理 - 验证指南

## 快速验证步骤

### 1. 启动应用

```bash
cd log-analyzer
npm run tauri dev
```

### 2. 测试导入任务

1. 点击 "Workspaces" 页面
2. 点击 "Import Folder" 或 "Import Archive"
3. 选择一个文件夹或压缩包
4. 观察 "Tasks" 页面

**预期行为**：
- ✅ 任务立即出现在列表中
- ✅ 进度条实时更新（0% → 100%）
- ✅ 任务完成后显示绿色勾号
- ✅ **3 秒后任务自动消失**（关键！）

### 3. 测试刷新任务

1. 在 "Workspaces" 页面选择一个已导入的工作区
2. 点击刷新按钮
3. 观察 "Tasks" 页面

**预期行为**：
- ✅ 刷新任务出现
- ✅ 进度更新（扫描 → 分析 → 保存）
- ✅ 完成后 **3 秒自动消失**

### 4. 测试失败任务

1. 尝试导入一个不存在的路径（手动触发失败）
2. 观察 "Tasks" 页面

**预期行为**：
- ✅ 任务显示红色错误图标
- ✅ **10 秒后自动消失**（失败任务保留更久）

## 控制台日志验证

打开浏览器开发者工具（F12），查看控制台输出：

### 后端日志（Rust）

```
[TaskManager] Actor started
[TaskManager] Task created: import-xxx
[TaskManager] Task updated: import-xxx (progress: 50%)
[TaskManager] Task completed: import-xxx
[TaskManager] Auto-removed expired task: import-xxx (status: Completed)
```

### 前端日志（TypeScript）

```
[TaskManager] Auto-removing task: import-xxx
```

## 性能验证

### 内存占用

1. 打开任务管理器
2. 导入 10 个工作区
3. 等待所有任务完成
4. 等待 5 分钟

**预期行为**：
- ✅ 内存占用稳定，不增长
- ✅ 任务列表为空（所有任务已清理）

### CPU 占用

**预期行为**：
- ✅ 空闲时 CPU 占用 < 1%
- ✅ 定时清理不会导致 CPU 峰值

## 边界情况测试

### 1. 快速连续导入

1. 快速导入 5 个工作区
2. 观察任务列表

**预期行为**：
- ✅ 所有任务都正确显示
- ✅ 没有重复任务
- ✅ 完成后依次自动消失

### 2. 应用重启

1. 导入一个工作区
2. 在任务运行中关闭应用
3. 重新启动应用

**预期行为**：
- ✅ 应用正常启动
- ✅ 旧任务不会残留

### 3. 长时间运行

1. 启动应用
2. 运行 1 小时
3. 期间导入多个工作区

**预期行为**：
- ✅ 应用稳定运行
- ✅ 没有内存泄漏
- ✅ 任务列表始终保持整洁

## 故障排查

### 问题：任务不自动消失

**检查项**：
1. 查看后端日志，确认 TaskManager 已初始化
2. 查看前端日志，确认监听了 `task-removed` 事件
3. 检查配置：`completed_task_ttl` 是否正确设置

**解决方案**：
```bash
# 重新编译后端
cargo build --manifest-path log-analyzer/src-tauri/Cargo.toml

# 重新启动应用
npm run tauri dev
```

### 问题：任务重复出现

**检查项**：
1. 查看前端 `addTaskIfNotExists` 是否正确调用
2. 检查任务 ID 是否唯一

**解决方案**：
- 确保使用 `addTaskIfNotExists` 而不是 `addTask`

### 问题：任务消失太快/太慢

**调整配置**：

编辑 `log-analyzer/src-tauri/src/lib.rs`：

```rust
let task_manager_config = task_manager::TaskManagerConfig {
    completed_task_ttl: 5,  // 改为 5 秒
    failed_task_ttl: 15,     // 改为 15 秒
    cleanup_interval: 1,
};
```

## 成功标准

✅ **所有测试通过**：
- 导入任务 3 秒后自动消失
- 刷新任务 3 秒后自动消失
- 失败任务 10 秒后自动消失
- 无内存泄漏
- 无重复任务
- 控制台日志正常

## 下一步

如果所有测试通过，可以：

1. 提交代码：
```bash
git add .
git commit -m "feat(task): implement Actor-based task lifecycle management

- Use Tokio Actor Pattern for task management
- Auto-cleanup completed tasks after 3 seconds
- Auto-cleanup failed tasks after 10 seconds
- Add task-removed event for frontend synchronization
- Improve user experience by keeping task list clean
"
```

2. 更新文档
3. 部署到生产环境

## 参考文档

- [TASK_LIFECYCLE_MANAGEMENT_IMPLEMENTATION.md](./TASK_LIFECYCLE_MANAGEMENT_IMPLEMENTATION.md) - 详细实现说明
- [Tokio Actors Pattern](https://tokio.rs/tokio/topics/actors) - 官方文档
- [Actix Framework](https://actix.rs/) - Actor 框架参考

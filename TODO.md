# 未完成任务清单 (TODO)

> 本文档记录代码中所有未完成的任务、TODO 注释和待实现功能。
> 最后更新: 2026-02-11

---

## 🎯 任务优先级说明

- **P0 - 高优先级**: 核心功能缺失，影响用户体验
- **P1 - 中优先级**: 性能优化或架构改进
- **P2 - 低优先级**: 代码清理或文档完善

---

## 📋 Rust 后端未完成任务

### P0 - 高优先级

#### 1. 搜索历史记录功能
**位置**: `src/commands/` (缺失)

**描述**: 前端调用 `add_search_history` 命令，但后端未实现

**影响**: 用户搜索历史无法保存

**相关代码**:
```typescript
// src/pages/SearchPage.tsx:277
invoke('add_search_history', {
  query: query.trim(),
  workspaceId: activeWorkspace.id,
  resultCount: count,
}).catch(err => {
  logger.error('Failed to save search history:', getFullErrorMessage(err));
});
```

**建议实现**:
- [ ] 创建 `src/commands/search_history.rs`
- [ ] 实现 `add_search_history` 命令
- [ ] 实现 `get_search_history` 命令
- [ ] 实现 `clear_search_history` 命令
- [ ] 在 `lib.rs` 中注册命令

---

### P1 - 中优先级

#### 2. 任务管理器性能指标
**位置**: `src/commands/performance.rs:274-284`

**描述**: `get_task_manager_metrics` 返回默认值，未获取实际任务数据

**相关代码**:
```rust
fn get_task_manager_metrics(_state: &AppState) -> TaskMetrics {
    // 简化处理：返回默认值
    // TODO: 通过异步消息获取实际的任务管理器指标
    TaskMetrics {
        total: 0,
        running: 0,
        completed: 0,
        failed: 0,
        average_duration: 0,
    }
}
```

**建议实现**:
- [ ] 通过 Tauri 事件系统与 TaskManager 通信
- [ ] 获取真实的任务统计数据
- [ ] 计算平均执行时间

---

#### 3. 索引指标数据
**位置**: `src/commands/performance.rs:304-316`

**描述**: `get_index_metrics` 使用存储数量作为文件计数

**相关代码**:
```rust
fn get_index_metrics(state: &AppState) -> IndexMetrics {
    // 简化处理：使用存储数量作为文件计数
    // TODO: 从 MetadataStore 获取实际的文件统计信息
    IndexMetrics {
        total_files: store_count,
        indexed_files: store_count,
        total_size: 0,
        index_size: 0,
    }
}
```

**建议实现**:
- [ ] 在 MetadataStore 中添加文件统计方法
- [ ] 返回实际的总文件大小和索引大小

---

#### 4. 工作区名称读取
**位置**: `src/commands/workspace.rs:737`

**描述**: 工作区状态返回中使用 ID 作为名称

**相关代码**:
```rust
Ok(WorkspaceStatusResponse {
    id: workspaceId.clone(),
    name: workspaceId, // TODO: 从配置中读取实际名称
    status: "READY".to_string(),
    ...
})
```

**建议实现**:
- [ ] 在 Workspace 创建时保存显示名称
- [ ] 从元数据中读取名称

---

### P2 - 低优先级（架构清理）

#### 5. DDD 架构模块缺失
**位置**: `src/infrastructure/mod.rs`, `src/domain/mod.rs` 等

**描述**: 部分 DDD 架构模块文件缺失，已暂时注释

**缺失模块**:
- `infrastructure/persistence` - 持久化模块
- `infrastructure/messaging` - 消息模块
- `infrastructure/external` - 外部服务集成
- `domain/shared/value_objects` - 值对象模块
- `domain/shared/specifications` - 规范模块
- `domain/log_analysis/services` - 日志分析服务
- `domain/log_analysis/events` - 日志分析事件
- `domain/log_analysis/repositories` - 日志分析仓储

**建议**: 根据实际需求选择性实现，或清理未使用的模块引用

---

#### 6. 插件系统集成未完成
**位置**: `src/application/services/mod.rs:5,14`

**描述**: 插件管理器已创建但未完全集成

**相关代码**:
```rust
// use crate::application::plugins::PluginManager; // TODO: 插件系统暂未完全集成
// plugins: Arc<PluginManager>, // TODO: 插件系统暂未完全集成
// TODO: 通过插件处理搜索查询
```

**建议**: 等待插件系统需求明确后完成集成

---

#### 7. 配置文件加载未实现
**位置**: `src/infrastructure/config/mod.rs:256-258`

**描述**: 配置加载返回默认值，未实际加载文件

**相关代码**:
```rust
// TODO: 实际实现文件加载 (暂时返回默认配置)
// let content = std::fs::read_to_string(path)?;
// let config: Self = match path.extension()...
```

**建议**: 根据实际配置文件格式实现加载逻辑

---

#### 8. OpenTelemetry 集成
**位置**: `src/monitoring/mod.rs:31-32`

**描述**: tracing_opentelemetry 模块缺失

**相关代码**:
```rust
// TODO: tracing_opentelemetry 模块缺失，暂时注释
// .with(tracing_opentelemetry::layer());
```

**建议**: 根据遥测需求选择实现方案

---

## 📋 React 前端未完成任务

### P1 - 中优先级

#### 1. 远程错误追踪集成
**位置**: `src/components/ErrorBoundary.tsx`

**描述**: 全局错误边界预留了 Sentry 集成接口

**相关代码**:
```typescript
// TODO: 集成远程错误追踪服务（如 Sentry）
// 生产环境可以考虑：
// if (process.env.NODE_ENV === 'production') {
//   Sentry.captureException(error);
// }
```

**出现位置**:
- `ErrorBoundary.tsx:281` - initGlobalErrorHandlers
- `ErrorBoundary.tsx:562` - CompactErrorFallback
- `ErrorBoundary.tsx:605` - PageErrorFallback

**建议实现**:
- [ ] 安装 Sentry SDK: `npm install @sentry/react`
- [ ] 配置 Sentry DSN
- [ ] 在生产环境启用
- [ ] 添加用户信息和上下文

---

### P2 - 低优先级

#### 2. React 非控制输入警告
**位置**: React 控制台警告

**描述**: 某些输入组件从非控制状态变为控制状态

**建议**: 检查所有 Input 组件，确保初始值正确设置

---

## 📊 统计摘要

| 类别 | 数量 | 详情 |
|------|------|------|
| **Rust 后端 TODO** | 8 项 | P0: 1, P1: 3, P2: 4 |
| **前端 TODO** | 2 项 | P1: 1, P2: 1 |
| **总计** | 10 项 | - |

---

## 🔗 相关链接

- [开发指南](docs/development/AGENTS.md)
- [架构文档](log-analyzer/src-tauri/CLAUDE.md)
- [前端文档](log-analyzer/src/CLAUDE.md)
- [CHANGELOG](CHANGELOG.md)

---

> **注意**: 本文档会随着代码变更持续更新。在实施任务前请先检查最新状态。

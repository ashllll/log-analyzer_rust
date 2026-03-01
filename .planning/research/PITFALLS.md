# Pitfalls Research: Flutter + Rust Backend Integration

**Domain:** Flutter Desktop + Rust Backend FFI Integration
**Researched:** 2026-02-28
**Confidence:** MEDIUM

Based on analysis of the codebase, FFI documentation, and community patterns.

---

## Critical Pitfalls

### Pitfall 1: Field Name Mismatch (snake_case vs camelCase)

**What goes wrong:**
Flutter前端发送 `taskId`，但 Rust 后端期望 `task_id`，导致序列化/反序列化失败，搜索功能完全不可用。

**Why it happens:**
- Dart 惯例是 camelCase (`taskId`, `workspaceId`)
- Rust 惯例是 snake_case (`task_id`, `workspace_id`)
- flutter_rust_bridge 默认不转换字段名

**How to avoid:**
1. **统一约定**: 在整个项目中强制使用 snake_case
2. **Dart 端**: 所有与 Rust 通信的模型使用 snake_case 字段名
3. **Rust 端**: 所有对外暴露的结构体使用 snake_case

```dart
// ❌ 错误 - Dart 惯例
class SearchResult {
  String searchId;      // camelCase
  String workspaceId;   // camelCase
}

// ✅ 正确 - 与 Rust 保持一致
class SearchResult {
  String search_id;     // snake_case
  String workspace_id;  // snake_case
}
```

**Warning signs:**
- 编译通过但运行时数据为 null
- 序列化错误日志: "missing field `task_id`"
- 前后端 API 联调时数据不一致

**Phase to address:**
- 架构设计阶段 (Phase 1-2)

---

### Pitfall 2: FFI Boundary Error Handling Lost

**What goes wrong:**
Rust 中的 `Result<T, E>` 错误在 FFI 边界丢失，Flutter 端只收到 panic 或 null，无法获取具体错误信息。

**Why it happens:**
- flutter_rust_bridge 2.x 将 Rust panic 转换为 Dart 异常
- 但错误详情可能丢失或格式不正确
- 异步函数错误传播机制不完善

**How to avoid:**
1. 使用 `thiserror` 定义结构化错误类型
2. 在 Rust 端使用 `?` 传播错误，确保 panic 信息包含上下文
3. Flutter 端实现全局错误边界 (ErrorWidget)

```rust
// ✅ 正确 - 错误包含上下文
#[derive(thiserror::Error, Debug)]
pub enum AppError {
    #[error("Workspace not found: {0}")]
    WorkspaceNotFound(String),
}

#[tauri::command]
pub fn get_workspace(id: String) -> Result<Workspace, String> {
    // 使用 ? 自动转换，错误信息包含 id
    workspace_repo.find(&id).map_err(|e| format!("Failed to get workspace {}: {}", id, e))
}
```

**Warning signs:**
- Flutter 端收到笼统的 "FFI error" 消息
- 无法区分不同类型的错误 (网络/文件/权限)
- 错误日志缺少调试信息

**Phase to address:**
- API 设计阶段 (Phase 2-3)

---

### Pitfall 3: Blocking the Main Thread with FFI Calls

**What goes wrong:**
在 Flutter 主线程执行同步 FFI 调用，导致 UI 冻结，尤其是在处理大文件或复杂搜索时。

**Why it happens:**
- 同步函数 (`#[frb(sync)]`) 在主线程执行
- Dart 的 isolate 不与 Rust 线程直接对应
- 没有实现真正的异步流

**How to avoid:**
1. **优先使用异步函数**: Rust 端使用 `async fn`，Dart 端使用 `Future`
2. **实现后台任务**: 使用 Flutter isolates 处理耗时操作
3. **流式结果**: 对于大结果集，使用分页或流式返回

```rust
// ✅ 正确 - 异步函数
#[frb]
pub async fn search_logs(query: String) -> SearchResult {
    // 异步执行，不阻塞主线程
}

// ❌ 避免 - 同步函数用于耗时操作
#[frb(sync)]
pub fn search_logs_blocking(query: String) -> SearchResult {
    // 会阻塞调用线程
}
```

**Warning signs:**
- UI 在搜索时卡顿
- "Frame skipped" 警告
- 大文件操作时应用无响应

**Phase to address:**
- 性能优化阶段 (Phase 4-5)

---

### Pitfall 4: State Synchronization Race Conditions

**What goes wrong:**
Flutter 端状态与 Rust 后端状态不一致，导致显示过期数据或任务状态错误。

**Why it happens:**
- 事件驱动更新 vs 轮询更新机制冲突
- 并发操作导致版本冲突
- 网络中断/重连时状态丢失

**How to avoid:**
1. **实现版本号机制**: 每个状态变更携带递增版本号
2. **幂等事件处理**: 相同版本号的事件重复处理不产生副作用
3. **乐观更新 + 回滚**: 先更新 UI，后验证，失败时回滚

```dart
// ✅ 正确 - 版本号校验
class WorkspaceState {
  final String id;
  final int version;

  bool canUpdate(int newVersion) => newVersion > version;

  WorkspaceState apply(WorkspaceEvent event) {
    if (event.version <= version) return this; // 忽略过期事件
    return applyEvent(event);
  }
}
```

**Warning signs:**
- 任务进度显示不正确
- 搜索结果列表闪烁或重复
- "Version conflict" 错误日志

**Phase to address:**
- 状态管理设计阶段 (Phase 3)

---

### Pitfall 5: Memory Leaks at FFI Boundary

**What goes wrong:**
Rust 分配的内存在 Dart 端未正确释放，或反之，导致内存持续增长。

**Why it happens:**
- 跨语言内存管理语义不同
- Rust 的 `Box<T>` 在 Dart 端需要手动 drop
- 不对称的 alloc/dealloc

**How to avoid:**
1. **使用 opaque 类型**: FRB 的 `#[frb(opaque)]` 自动处理生命周期
2. **避免裸指针传递**: 使用智能指针和 FRB 的所有权语义
3. **添加内存监控**: 定期检查内存使用趋势

```rust
// ✅ 正确 - 使用 FRB 管理生命周期
#[frb(opaque)]
pub struct SearchContext {
    // FRB 自动处理 drop
}

#[frb]
pub fn create_context() -> SearchContext {
    SearchContext::new()
}

// ❌ 避免 - 手动内存管理
pub fn create_raw_pointer() -> *mut SearchContext {
    Box::into_raw(Box::new(SearchContext::new()))
}
```

**Warning signs:**
- 内存持续增长不释放
- 应用空闲时内存占用高
- 长时间运行后应用变慢

**Phase to address:**
- 内存管理设计阶段 (Phase 2-3)

---

## Technical Debt Patterns

| Shortcut | Immediate Benefit | Long-term Cost | When Acceptable |
|----------|-------------------|----------------|------------------|
| 使用 `String` 替代结构化错误 | 简单快速 | 错误类型丢失，调试困难 | MVP 阶段临时 |
| 跳过 FFI 集成测试 | 节省时间 | 边界问题在生产暴露 | - |
| 同步 FFI 调用简化代码 | 代码简洁 | UI 卡顿，用户体验差 | - |
| 手动管理内存引用计数 | 细微性能提升 | 内存泄漏风险 | - |
| 忽略版本号的幂等性 | 快速实现 | 状态同步 Bug | - |

---

## Integration Gotchas

| Integration | Common Mistake | Correct Approach |
|-------------|----------------|------------------|
| **flutter_rust_bridge** | 字段名不转换 | 统一 snake_case |
| **Tauri 事件系统** | 事件版本号重复 | 维护单调递增版本号 |
| **HTTP API** | 超时未处理 | 实现重试 + 超时回退 |
| **文件选择器** | 路径未验证 | 白名单验证目录 |
| **热更新** | 状态未持久化 | 分离瞬态/持久状态 |

---

## Performance Traps

| Trap | Symptoms | Prevention | When It Breaks |
|------|----------|------------|----------------|
| 大型 Vector 跨 FFI 边界 | 复制耗时，内存翻倍 | 使用流式/分页 API | 1000+ 结果集 |
| 同步搜索调用 | UI 冻结 100ms+ | 异步 + isolates | 任何搜索操作 |
| 未缓存的正则表达式 | CPU 占用飙升 | RegexCache 预编译 | 正则搜索 |
| 事件流未限流 | 内存暴涨 | 背压控制 | 文件监听高频率更新 |

---

## Security Mistakes

| Mistake | Risk | Prevention |
|---------|------|------------|
| 未验证的文件路径 | 路径遍历攻击 | 白名单 + 路径规范化 |
| 插件动态加载 | 恶意代码执行 | 目录白名单 + ABI 验证 |
| FFI 边界数据未校验 | 缓冲区溢出 | 输入验证 + 类型检查 |
| 敏感数据日志 | 信息泄露 | 脱敏处理 + 日志级别控制 |

---

## UX Pitfalls

| Pitfall | User Impact | Better Approach |
|---------|-------------|------------------|
| 搜索无结果无提示 | 用户困惑 | 空结果页面 + 搜索建议 |
| 长时间操作无反馈 | 用户以为卡死 | 进度指示器 + 取消按钮 |
| 错误信息不友好 | 用户无法理解问题 | 人类可读的错误消息 |
| 状态突变无动画 | 界面闪烁 | 过渡动画 |

---

## "Looks Done But Isn't" Checklist

- [ ] **搜索功能**: 实现分页/流式加载了吗？全量加载大数据集会崩溃
- [ ] **错误处理**: Rust 错误正确映射到 Dart 异常了吗？测试各种边界情况
- [ ] **状态同步**: 版本号机制实现了吗？跳过会导致状态 Bug
- [ ] **内存管理**: 所有 FFI 资源正确释放了吗？运行长时间测试检查内存
- [ ] **异步处理**: 耗时操作不阻塞 UI 吗？测试大文件场景
- [ ] **取消机制**: 搜索/导入可取消吗？实现 AbortController
- [ ] **离线处理**: FFI 初始化失败有降级方案吗？HTTP API 备选

---

## Recovery Strategies

| Pitfall | Recovery Cost | Recovery Steps |
|---------|---------------|----------------|
| 字段名不匹配 | LOW | 重命名 Dart 模型字段，regenerate FRB |
| 状态同步 Bug | MEDIUM | 添加版本号，重启应用清除状态 |
| 内存泄漏 | HIGH | 重启应用，代码审计 FRB 边界 |
| FFI 崩溃 | MEDIUM | 添加错误边界，降级到 HTTP API |

---

## Pitfall-to-Phase Mapping

| Pitfall | Prevention Phase | Verification |
|---------|------------------|--------------|
| 字段名不一致 | Phase 1-2: 架构设计 | 代码审查 + 集成测试 |
| 错误处理丢失 | Phase 2-3: API 设计 | 单元测试 + 边界测试 |
| 阻塞主线程 | Phase 4-5: 性能优化 | 性能测试 + UI 响应测试 |
| 状态同步问题 | Phase 3: 状态管理 | 并发测试 + 事件测试 |
| 内存泄漏 | Phase 2-3: 内存设计 | 压力测试 + 内存分析 |
| 事件流问题 | Phase 3: 事件系统 | 集成测试 + 断网重连测试 |

---

## Sources

- flutter_rust_bridge 官方文档: https://cjycode.com/flutter_rust_bridge/
- FRB 状态管理指南: https://cjycode.com/flutter_rust_bridge/guides/how-to/stateful-rust
- FRB 错误处理: https://cjycode.com/flutter_rust_bridge/guides/how-to/report-error
- 项目现有 FFI 实现: `log-analyzer/src-tauri/src/ffi/bridge.rs`
- 项目现有代码规范: `.planning/codebase/CONCERNS.md`
- Rust FFI 最佳实践: https://microsoft.github.io/rust-guidelines/guidelines/ffi/

---

*Pitfalls research for: Flutter Desktop + Rust Backend Integration*
*Researched: 2026-02-28*

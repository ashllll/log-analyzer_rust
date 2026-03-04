# Phase 8: 状态管理 - Context

**Gathered:** 2026-03-04
**Status:** Ready for planning

<domain>
## Phase Boundary

使用 Riverpod 管理搜索历史和虚拟文件树的状态，为 Phase 9（高级搜索 UI）和 Phase 10（虚拟文件系统 UI）提供响应式数据源。实现 SearchHistoryProvider 和 VirtualFileTreeProvider，支持与 Rust 后端的 FFI 数据同步。

</domain>

<decisions>
## Implementation Decisions

### Provider 结构设计
- **Provider 粒度**: 功能级 Provider — SearchHistoryProvider 和 VirtualFileTreeProvider 各自独立
- **Provider 类型**: AsyncNotifierProvider — 支持异步数据加载，与后端 FFI 调用配合良好
- **状态模型**: Freezed 不可变类 — 类型安全，支持 copyWith，Riverpod 官方推荐
- **工作区绑定**: 参数传递 — Provider 接受 workspaceId 参数，切换工作区时重新加载数据
- **依赖注入**: Riverpod Ref — 使用 ref.watch/read 访问 BridgeService (FFI 服务)
- **初始化时机**: 懒加载 — UI 首次 watch/read 时自动初始化
- **状态分离**: 独立状态 — 每个 Provider 有独立的状态和加载状态，互不干扰
- **副作用处理**: 内部处理 — Provider 内部处理所有逻辑，外部只需 watch
- **文件树展开状态**: TreeController — 使用 flutter_fancy_tree_view2 的 TreeController，自带展开/折叠管理
- **历史排序**: 时间降序 — 最近搜索在最前面
- **LRU 限制执行**: 后端执行 — Provider 调用后端时由后端强制执行（最多100条），前端无需关心

### 数据同步策略
- **同步时机**: 自动同步 — 搜索完成后自动保存到历史，文件树切换工作区时自动加载
- **更新模式**: 乐观更新 — 先更新 UI，后同步后端，失败时回滚
- **冲突解决**: 后端数据优先 — 乐观更新失败时以数据库为准，前端同步后端数据
- **刷新触发**: 搜索完成后 + 文件导入后 + 手动刷新按钮 + 文件监控事件

### 缓存与内存管理
- **内存缓存**: Provider 自动缓存 — Riverpod Provider 自动管理缓存，窗口关闭时自动释放
- **缓存失效**: 工作区切换时失效 — 切换工作区时自动失效缓存，重新加载新工作区数据
- **文件内容缓存**: 只缓存结构 — Provider 只缓存文件树结构，文件内容按需从后端读取，不缓存
- **懒加载策略**: 实时懒加载 — 展开目录时实时从后端加载子节点，数据最新

### 错误处理模式
- **错误传播**: AsyncError — Provider 捕获错误并转换为 AsyncError，UI 层根据 hasError 显示错误状态
- **重试策略**: 自动重试 — 使用 Riverpod 的 retry 策略，自动重试 3 次
- **加载状态**: AsyncLoading — 使用 Riverpod 的 AsyncLoading 状态，UI 显示加载指示器
- **空状态处理**: UI 层处理 — Provider 返回空列表，UI 层负责显示空状态界面

</decisions>

<specifics>
## Specific Ideas

- 参考 Phase 7 已实现的 FFI API：`addSearchHistory`, `getSearchHistory`, `deleteSearchHistory`, `clearSearchHistory`, `getVirtualFileTree`, `getTreeChildren`
- 使用 flutter_fancy_tree_view2 实现 TreeController（已在 STATE.md 中确定）
- 搜索历史 LRU 限制：最多100条，30天过期（已在 STATE.md 中确定）

</specifics>

<deferred>
## Deferred Ideas

None — discussion stayed within phase scope

</deferred>

---

*Phase: 08-state-management*
*Context gathered: 2026-03-04*

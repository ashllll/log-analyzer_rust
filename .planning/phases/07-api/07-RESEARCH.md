# Phase 7: 后端 API 集成 - Research

**Researched:** 2026-03-04
**Domain:** Flutter FFI 与 Rust 后端集成
**Confidence:** HIGH

## Summary

Phase 7 目标是让 Flutter 应用通过 flutter_rust_bridge FFI 调用 Rust 后端的搜索历史和虚拟文件树 API。核心发现：

1. **Rust 后端已有完整实现**：搜索历史 (`SearchHistoryManager`)、虚拟文件树 (`VirtualTreeNode`)、多关键词组合搜索 (AND/OR/NOT)、正则表达式搜索均已在后端实现
2. **FFI 集成模式已建立**：使用 `flutter_rust_bridge` 2.x，在 `ffi/bridge.rs` 定义函数，在 Flutter 端 `bridge_service.dart` 封装调用
3. **Stream 流式传输**：FRB 2.x 不支持 `#[frb(stream)]`，需使用 Tauri 事件系统 (`app.emit`) 实现
4. **主要工作量**：将现有 Tauri 命令转换为 FFI 函数，生成 Dart 代码，封装 Flutter 服务方法

**Primary recommendation:** 扩展现有 FFI bridge 模式，将 `commands/search_history.rs` 和 `commands/virtual_tree.rs` 中的 Tauri 命令转换为 FFI 函数。

---

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions
- 使用细粒度方法设计 (如 `searchHistory.add`, `searchHistory.get`, `searchHistory.delete`)
- 虚拟文件树使用懒加载方式，按需加载子节点
- 支持批量删除操作
- FFI 调用失败时抛出异常，Flutter 端使用分类异常
- 后端重试策略：3次重试
- FFI 调用超时时间：10秒

### Claude's Discretion
- 使用 Tauri 事件系统实现 Stream 流式传输 (FRB 2.x 不支持 `#[frb(stream)]`)
- 分批大小：每批100条
- 启用 debounce (快速连续刷新只触发一次)

### Deferred Ideas (OUT OF SCOPE)
None — discussion stayed within phase scope
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|-----------------|
| 07-01 | 搜索历史 API 集成（添加、获取、删除、清空） | 后端已有 `SearchHistoryManager`，需转换为 FFI |
| 07-02 | 虚拟文件树 API 集成（获取树结构） | 后端已有 `get_virtual_file_tree` 命令，需转换为 FFI |
| 07-03 | 正则表达式搜索 API 集成 | 后端已支持 `SearchTerm.is_regex`，需暴露 FFI 接口 |
| 07-04 | 多关键词组合搜索 API 集成 | 后端已支持 `QueryOperator` (AND/OR/NOT)，需暴露 FFI 接口 |
</phase_requirements>

---

## Standard Stack

### Core
| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| `flutter_rust_bridge` | 2.11.1 | Flutter Rust FFI 桥接 | 项目已在使用，版本锁定 |
| `tauri` | 2.0.0 | 桌面应用框架 + 事件系统 | 项目已在使用 |
| `tokio` | 1.x | 异步运行时 | 项目已在使用 |

### Supporting
| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| `thiserror` | 1.0 | 错误处理 | 定义 FFI 错误类型 |
| `serde` | - | 序列化 | 已在项目中广泛使用 |

### Alternatives Considered
| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| FRB Stream | Tauri Events + MethodChannel | FRB 2.x 不支持 Stream，需使用 Tauri 事件 |
| 直接 FFI 调用 | HTTP API | 项目已优先使用 FFI，保持一致性 |

---

## Architecture Patterns

### Recommended Project Structure
```
log-analyzer/src-tauri/src/
├── ffi/
│   ├── bridge.rs           # FFI 函数定义 (扩展)
│   ├── types.rs            # FFI 类型定义 (扩展)
│   └── commands_bridge.rs  # 命令桥接实现
├── commands/
│   ├── search_history.rs   # 已有 Tauri 命令 (可直接复用)
│   └── virtual_tree.rs     # 已有 Tauri 命令 (可直接复用)
└── models/
    └── search_history.rs   # 已有模型定义

log-analyzer_flutter/lib/shared/services/
├── bridge_service.dart     # 扩展 FFI 封装方法
└── generated/ffi/
    └── bridge.dart         # 自动生成
```

### Pattern 1: FFI 函数扩展
**What:** 在 `ffi/bridge.rs` 添加新的 FFI 函数，使用 `#[frb(sync)]` 或 `#[frb]` 宏装饰

**When to use:** 需要从 Flutter 调用 Rust 功能时

**Example:**
```rust
// Rust 后端 - ffi/bridge.rs
#[frb(sync)]
pub fn add_search_history(
    query: String,
    workspace_id: String,
    result_count: i32,
) -> bool {
    // 复用现有命令逻辑
    unwrap_result(
        commands_bridge::ffi_add_search_history(query, workspace_id, result_count),
        "添加搜索历史失败"
    )
}
```

### Pattern 2: Flutter 端 FFI 封装
**What:** 在 `bridge_service.dart` 添加对应的封装方法

**When to use:** 需要在 Flutter 中调用 FFI 函数时

**Example:**
```dart
// Flutter 端 - bridge_service.dart
Future<bool> addSearchHistory({
  required String query,
  required String workspaceId,
  required int resultCount,
}) async {
  if (!isFfiEnabled) {
    throw FfiInitializationException('FFI not initialized');
  }
  try {
    final result = ffi.addSearchHistory(
      query: query,
      workspaceId: workspaceId,
      resultCount: resultCount,
    );
    return result.ok;
  } catch (e) {
    debugPrint('addSearchHistory error: $e');
    rethrow;
  }
}
```

### Pattern 3: Tauri 事件流 (替代 FRB Stream)
**What:** 使用 Tauri 的 `app.emit()` 发送事件，Flutter 端通过 `MethodChannel` 监听

**When to use:** 需要流式传输大量数据（如虚拟文件树懒加载）

**Example:**
```rust
// Rust 后端 - commands/virtual_tree.rs
async fn get_tree_children(
    app: AppHandle,
    workspace_id: String,
    parent_path: String,
) -> Result<Vec<VirtualTreeNode>, String> {
    // ... 获取子节点逻辑
    // 发送分批数据
    for chunk in nodes.chunks(100) {
        let _ = app.emit("tree-chunk", chunk);
    }
}
 Anti-Patterns to Avoid
-```

### **不要手动实现 FFI 桥接**：使用 `flutter_rust_bridge` 自动生成代码
- **不要混用 Tauri invoke 和 FFI**：保持统一的调用模式
- **不要忽略错误处理**：FFI 调用失败会转换为 Dart 异常，需要捕获处理

---

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| FFI 序列化 | 手写 Rust-Dart 类型转换 | flutter_rust_bridge 自动生成 | 保证类型安全，减少维护成本 |
| 错误传播 | 返回错误码 | panic + FRB 异常转换 | FRB 2.x 最佳实践 |
| 流式数据 | 等待全部数据 | Tauri Events 分批发送 | 避免内存爆炸，支持大数据集 |

**Key insight:** 项目已建立完整的 FFI 集成模式，应遵循现有模式扩展而非另起炉灶。

---

## Common Pitfalls

### Pitfall 1: FFI 初始化时机不当
**What goes wrong:** Flutter 端 FFI 未初始化就调用方法，导致崩溃
**Why it happens:** FFI 需要在首次使用时初始化（延迟加载）
**How to avoid:** 使用 `BridgeService.instance.initialize()` 在应用启动时初始化
**Warning signs:** `FFI_NOT_INITIALIZED` 错误

### Pitfall 2: Snake_case vs camelCase 命名不一致
**What goes wrong:** Rust 使用 snake_case，Flutter 使用 camelCase
**Why it happens:** Rust 后端使用 snake_case，Flutter 惯例使用 camelCase
**How to avoid:** 使用 `#[allow(non_snake_case)]` 并在 Dart 端使用正确的命名
**Warning signs:** 序列化错误，参数为 null

### Pitfall 3: Stream 数据量过大
**What goes wrong:** 虚拟文件树一次性加载所有节点，导致内存溢出
**Why it happens:** 未实现懒加载或分批传输
**How to avoid:** 使用 Tauri 事件分批发送（每批100条），Flutter 端实现增量更新
**Warning signs:** 加载缓慢，内存占用持续增长

### Pitfall 4: 错误处理不一致
**What goes wrong:** 部分函数返回错误码，部分抛出异常
**Why it happens:** 没有统一的错误处理规范
**How to avoid:** 遵循 CONTEXT.md 决策：FFI 失败时抛出异常，使用统一的重试策略
**Warning signs:** 偶发性的静默失败

---

## Code Examples

### 搜索历史 FFI 函数定义 (Rust)
```rust
// 位置: log-analyzer/src-tauri/src/ffi/bridge.rs

/// 添加搜索历史记录
#[frb(sync)]
pub fn add_search_history(
    query: String,
    workspace_id: String,
    result_count: i32,
) -> bool {
    unwrap_result(
        commands_bridge::ffi_add_search_history(query, workspace_id, result_count as usize),
        "添加搜索历史失败"
    )
}

/// 获取搜索历史记录
#[frb(sync)]
pub fn get_search_history(
    workspace_id: Option<String>,
    limit: Option<i32>,
) -> Vec<SearchHistoryData> {
    unwrap_result(
        commands_bridge::ffi_get_search_history(workspace_id, limit.map(|l| l as usize)),
        "获取搜索历史失败"
    )
}

/// 删除搜索历史记录
#[frb(sync)]
pub fn delete_search_history(id: String) -> bool {
    unwrap_result(
        commands_bridge::ffi_delete_search_history(id),
        "删除搜索历史失败"
    )
}

/// 清空搜索历史
#[frb(sync)]
pub fn clear_search_history(workspace_id: Option<String>) -> i32 {
    unwrap_result(
        commands_bridge::ffi_clear_search_history(workspace_id),
        "清空搜索历史失败"
    )
}
```

### 虚拟文件树 FFI 函数定义 (Rust)
```rust
// 位置: log-analyzer/src-tauri/src/ffi/bridge.rs

/// 获取虚拟文件树 (懒加载根节点)
#[frb(sync)]
pub fn get_virtual_file_tree(workspace_id: String) -> Vec<VirtualTreeNodeData> {
    unwrap_result(
        commands_bridge::ffi_get_virtual_file_tree(workspace_id),
        "获取虚拟文件树失败"
    )
}

/// 获取子节点 (懒加载)
#[frb(sync)]
pub fn get_tree_children(
    workspace_id: String,
    parent_path: String,
) -> Vec<VirtualTreeNodeData> {
    unwrap_result(
        commands_bridge::ffi_get_tree_children(workspace_id, parent_path),
        "获取子节点失败"
    )
}
```

### 正则表达式搜索 FFI 函数定义 (Rust)
```rust
// 位置: log-analyzer/src-tauri/src/ffi/bridge.rs

/// 执行正则表达式搜索
#[frb(sync)]
pub fn search_regex(
    pattern: String,
    workspace_id: Option<String>,
    max_results: i32,
    case_sensitive: bool,
) -> String {
    unwrap_result(
        commands_bridge::ffi_search_regex(
            pattern,
            workspace_id,
            max_results,
            case_sensitive,
        ),
        "正则表达式搜索失败"
    )
}

/// 验证正则表达式语法
#[frb(sync)]
pub fn validate_regex(pattern: String) -> RegexValidationResult {
    commands_bridge::ffi_validate_regex(pattern)
}
```

### 多关键词组合搜索 FFI 函数定义 (Rust)
```rust
// 位置: log-analyzer/src-tauri/src/ffi/bridge.rs

/// 执行结构化搜索 (支持 AND/OR/NOT)
#[frb(sync)]
pub fn search_structured(
    query: StructuredSearchQuery,
    workspace_id: Option<String>,
    max_results: i32,
) -> String {
    unwrap_result(
        commands_bridge::ffi_search_structured(query, workspace_id, max_results),
        "结构化搜索失败"
    )
}
```

---

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| Tauri invoke | FFI (flutter_rust_bridge) | v1.0 重构 | 性能提升，避免 HTTP 序列化开销 |
| 同步阻塞调用 | async/await | v1.0 重构 | 更好的并发处理 |
| 手动序列化 | 自动代码生成 | v1.0 重构 | 减少错误，保持类型同步 |
| FRB Stream | Tauri Events | FRB 2.x 限制 | 实现流式数据传输 |

**Deprecated/outdated:**
- Tauri 命令模式 (commands/*.rs) 仍可用，但新功能优先使用 FFI

---

## Open Questions

1. **搜索历史是否需要持久化存储？**
   - What we know: 当前 `SearchHistoryManager` 是内存存储，应用重启后丢失
   - What's unclear: 是否需要 SQLite 持久化？CONTEXT.md 未明确
   - Recommendation: Phase 7 暂不实现持久化，后续 Phase 考虑

2. **虚拟文件树增量更新策略？**
   - What we know: CONTEXT.md 提到 "文件树刷新采用增量更新"
   - What's unclear: 具体实现方式（事件监听 vs 轮询）
   - Recommendation: 使用 Tauri 文件监听事件，变化时推送更新

3. **是否需要实现搜索历史批量删除？**
   - What we know: CONTEXT.md 提到 "支持批量删除操作"
   - What's unclear: 批量删除的标识方式（ID 列表 vs 条件）
   - Recommendation: 添加 `delete_search_histories(ids: Vec<String>)` 方法

---

## Validation Architecture

### Test Framework
| Property | Value |
|----------|-------|
| Framework | Rust: `rstest`, `proptest` / Flutter: `flutter_test` |
| Config file | `Cargo.toml` (Rust), `pubspec.yaml` (Flutter) |
| Quick run command | `cargo test --lib --bins` (Rust), `flutter test` (Flutter) |
| Full suite command | `cargo test --all-features` (Rust), `flutter test --integration` (Flutter) |

### Phase Requirements -> Test Map
| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| 07-01 | 搜索历史 add/get/delete/clear | Unit | `cargo test search_history` | Yes (models/search_history.rs) |
| 07-02 | 虚拟文件树获取 | Unit | `cargo test virtual_tree` | Yes (commands/virtual_tree.rs) |
| 07-03 | 正则表达式搜索 | Unit | `cargo test regex_engine` | Yes (services/regex_engine.rs) |
| 07-04 | 多关键词组合搜索 | Unit | `cargo test query_planner` | Yes (services/query_planner.rs) |
| FFI-01 | FFI 桥接函数可用 | Integration | `flutter test bridge_test.dart` | No (需创建) |

### Sampling Rate
- **Per task commit:** `cargo test --lib --bins` + `flutter test`
- **Per wave merge:** `cargo test --all-features` + `flutter test --integration`
- **Phase gate:** Full suite green before `/gsd:verify-work`

### Wave 0 Gaps
- [ ] `log-analyzer_flutter/test/bridge_service_test.dart` — FFI 集成测试
- [ ] `log-analyzer/src-tauri/tests/ffi_bridge_test.rs` — Rust FFI 导出测试
- [ ] Flutter test dependency: `flutter_rust_bridge` 测试助手

---

## Sources

### Primary (HIGH confidence)
- 项目现有代码分析：`ffi/bridge.rs`, `ffi/types.rs`, `commands/search_history.rs`, `commands/virtual_tree.rs`
- Flutter 项目代码：`bridge_service.dart`, `generated/ffi/bridge.dart`

### Secondary (MEDIUM confidence)
- flutter_rust_bridge 官方文档 (基于代码注释中的版本说明 2.11.1)

### Tertiary (LOW confidence)
- 无

---

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH — 项目已在生产环境使用 flutter_rust_bridge 2.x
- Architecture: HIGH — 基于现有代码模式，项目规范明确
- Pitfalls: HIGH — 基于代码注释和现有实现的已知问题

**Research date:** 2026-03-04
**Valid until:** 2026-04-04 (30 days for stable API)

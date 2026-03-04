# Phase 5: 实时监控 - Research

**Researched:** 2026-03-03
**Domain:** File System Watching + Real-time Index Update + Flutter UI Integration
**Confidence:** HIGH

## Summary

Phase 5 requires implementing real-time file monitoring that automatically updates the search index when files change. The project already has a solid foundation: Rust backend uses `notify` crate for file watching with incremental index updates via Tantivy, and Flutter frontend has BridgeService methods for start/stop watching and EventStreamService for handling file change events. The main work involves connecting these existing components with proper UI controls (toolbar button, status panel) and implementing the debouncing, rate limiting, and queue management specified in the user decisions.

**Primary recommendation:** Leverage existing Rust file watcher and Flutter event stream infrastructure, add UI components for monitoring control and status display, implement queue-based change processing with debouncing.

---

<user_constraints>

## User Constraints (from CONTEXT.md)

### Locked Decisions

**监控控制 UI:**
- 工具栏按钮启用/禁用文件监控
- 按工作区独立控制（一个工作区一个监控实例）
- 眼睛图标根据状态变化颜色（绿色=监控中，红色=停止）
- 应用重启后默认关闭，需要手动启用

**文件变更检测:**
- 监听全部事件：创建、修改、删除
- 使用工作区现有忽略规则（.gitignore 等模式）
- 500ms 防抖处理连续变更
- 智能重命名处理（检测为重命名事件，更新索引路径）
- 递归监控所有子目录
- 跟踪符号链接指向的目标
- 限流处理：每秒最多处理 10 个文件变更，超出则排队
- 全部处理所有文件（包括超大文件 >100MB）
- 遇到无权限访问的目录：跳过并记录警告日志，继续监控其他目录

**索引更新行为:**
- 增量更新：只更新变化的条目，不重建整个索引
- 后台异步处理，不阻塞用户界面
- 自动刷新结果列表（无需用户手动刷新）
- 智能冲突处理：文件已删除则从索引移除，未找到则跳过
- 队列处理多个变更：依次处理
- 更新失败时：重试3次后跳过，记录日志
- 索引更新完成时：状态栏显示简短消息
- 启用监控时：启动时同步现有文件
- 更新队列限制最大长度，超出则丢弃旧请求

**状态显示:**
- 完整信息显示：监控状态、活动指示器、事件计数
- 专门的状态面板显示（不在工具栏或侧边栏）
- 实时显示处理的事件数和待处理数
- 显示正在监控的目录和文件数量

### Claude's Discretion

- 状态面板的具体布局设计
- 状态面板的打开/关闭动画
- 队列最大长度的具体数值
- 重试间隔时间
- 状态栏消息的具体文案

### Deferred Ideas (OUT OF SCOPE)

- 监控历史记录（查看过去的变更）- 未来阶段
- 监控告警规则（特定文件变化时通知）- 未来阶段

</user_constraints>

---

<phase_requirements>

## Phase Requirements

| ID | Description | Research Support |
|----|-------------|-----------------|
| MON-01 | 用户可以启用文件监控 | Existing Rust `start_watch` command + Flutter BridgeService + UI button |
| MON-02 | 文件变化时自动更新索引 | Existing Rust event emission + Tantivy incremental update + event handling |
| MON-03 | 用户可以查看监控状态 | New Flutter status panel component + monitoring state provider |

</phase_requirements>

---

## Standard Stack

### Core

| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| `notify` | 6.1+ | File system watching | Rust ecosystem standard, cross-platform |
| `flutter_rust_bridge` | 2.x | FFI between Flutter and Rust | Project standard |
| `Riverpod` | latest | State management | Project standard |
| `Tantivy` | 0.22 | Full-text search index | Project standard |

### Supporting

| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| `tokio` | 1.x | Async runtime | For async file processing |
| `crossbeam` | 0.8 | Channel/queue | For change event queuing |
| `governor` | 0.6 | Rate limiting | For 10 files/sec limit |

### Existing Infrastructure

| Component | Location | Status |
|-----------|----------|--------|
| `start_watch` command | `commands/watch.rs` | Implemented |
| `stop_watch` command | `commands/watch.rs` | Implemented |
| `isWatching` command | FFI bridge | Implemented |
| `file-changed` event | `commands/watch.rs:100` | Emits on file change |
| `new-logs` event | `services/file_watcher.rs:299` | Emits on new log entries |
| BridgeService | `bridge_service.dart:385-435` | FFI methods exist |
| EventStreamService | `event_stream_service.dart` | Handles FileChanged events |

**Installation:**
```bash
# No new Rust dependencies needed - notify already in Cargo.toml
# Flutter: No new packages needed - Riverpod already used
```

---

## Architecture Patterns

### Recommended Project Structure

```
log-analyzer_flutter/lib/
├── features/
│   └── realtime_monitoring/
│       ├── presentation/
│       │   ├── pages/
│       │   │   └── monitoring_page.dart      # Main monitoring page
│       │   └── widgets/
│       │       ├── monitoring_toolbar_button.dart  # Eye icon button
│       │       ├── monitoring_status_panel.dart     # Status display panel
│       │       └── monitoring_queue_widget.dart    # Queue status
│       ├── providers/
│       │   └── monitoring_provider.dart      # Riverpod state
│       └── models/
│           └── monitoring_state.dart         # State model
```

### Pattern 1: Monitoring State Management

**What:** Use Riverpod to manage monitoring state per workspace

**When to use:** For reactive UI updates when monitoring state changes

**Example:**
```dart
// providers/monitoring_provider.dart
import 'package:flutter_riverpod/flutter_riverpod.dart';

/// Monitoring state for a workspace
class MonitoringState {
  final bool isActive;
  final int eventsProcessed;
  final int pendingCount;
  final int monitoredDirsCount;
  final int monitoredFilesCount;
  final DateTime? lastUpdate;
}

class MonitoringNotifier extends StateNotifier<MonitoringState> {
  MonitoringNotifier() : super(MonitoringState(
    isActive: false,
    eventsProcessed: 0,
    pendingCount: 0,
    monitoredDirsCount: 0,
    monitoredFilesCount: 0,
  ));

  void startMonitoring(String workspaceId) {
    // Call BridgeService.startWatch
    // Subscribe to fileChanged events
    state = state.copyWith(isActive: true);
  }

  void stopMonitoring(String workspaceId) {
    // Call BridgeService.stopWatch
    // Unsubscribe from events
    state = state.copyWith(isActive: false);
  }

  void onFileChanged(FileChangeEvent event) {
    state = state.copyWith(
      eventsProcessed: state.eventsProcessed + 1,
      lastUpdate: DateTime.now(),
    );
  }
}

final monitoringProvider = StateNotifierProvider.family<
  MonitoringNotifier,
  MonitoringState,
  String  // workspaceId
>((ref, workspaceId) => MonitoringNotifier());
```

### Pattern 2: Event-Driven Index Update

**What:** React to Rust file-changed events and trigger incremental index updates

**When to use:** When files change in the monitored workspace

**Example:**
```dart
// In search_page.dart or monitoring_provider.dart
void setupFileChangeListener(String workspaceId) {
  final eventService = ref.read(eventStreamServiceProvider);

  eventService.fileChanged.listen((event) {
    // Debounce 500ms handled in Rust or Dart?
    // User decision: 500ms debounce - implement in Dart layer

    // Add to processing queue
    monitoringNotifier.addToQueue(event);

    // Rate limit: 10 files/sec - use governor or manual
    processNextFromQueue();
  });
}
```

### Pattern 3: Queue-Based Change Processing

**What:** Queue file changes and process with rate limiting

**When to use:** When handling burst file changes

**Example:**
```dart
class ChangeQueue {
  final List<FileChangeEvent> _queue = [];
  static const int maxQueueLength = 1000;  // Claude's discretion
  static const int maxRatePerSecond = 10;
  static const Duration debounceDelay = Duration(milliseconds: 500);

  Timer? _debounceTimer;
  DateTime? _lastProcessTime;
  int _processedThisSecond = 0;

  void add(FileChangeEvent event) {
    if (_queue.length >= maxQueueLength) {
      _queue.removeAt(0);  // Drop oldest
    }
    _queue.add(event);

    // Reset debounce timer
    _debounceTimer?.cancel();
    _debounceTimer = Timer(debounceDelay, _processQueue);
  }

  void _processQueue() {
    while (_queue.isNotEmpty && _processedThisSecond < maxRatePerSecond) {
      final event = _queue.removeAt(0);
      _processEvent(event);
      _processedThisSecond++;
    }

    // Reset rate counter every second
    Future.delayed(Duration(seconds: 1), () {
      _processedThisSecond = 0;
    });
  }
}
```

### Anti-Patterns to Avoid

- **Don't rebuild entire index on each change** - Use incremental updates (already implemented in Rust)
- **Don't process changes synchronously** - Use background async processing
- **Don't ignore permission errors** - Log warning and continue as per user decision

---

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| File watching | Custom fs::watch implementation | `notify` crate | Already implemented, cross-platform, well-tested |
| Rate limiting | Manual counter/timer | `governor` crate | Industry standard, handles burst scenarios |
| Event debouncing | Manual Timer | `flutter_bloc` debouncer or custom | Simple enough to custom, but consider timer_utils |

**Key insight:** The Rust backend already has comprehensive file watching infrastructure. The main effort is in the Flutter frontend integration and UI.

---

## Common Pitfalls

### Pitfall 1: Event Stream Not Connected

**What goes wrong:** `file-changed` events from Rust never reach Flutter UI

**Why it happens:** EventStreamService uses polling, not actual Tauri event listening

**How to avoid:** Ensure Flutter app properly listens to Tauri events via FFI or add HTTP polling fallback

**Warning signs:** Monitoring shows "active" but no events processed

### Pitfall 2: Memory Leak from Event Listeners

**What goes wrong:** Event listeners accumulate, causing memory growth

**Why it happens:** Not disposing subscriptions when workspace changes

**How to avoid:** Use Riverpod's `ref.onDispose` to clean up listeners

**Warning signs:** Memory increases over time, especially with active monitoring

### Pitfall 3: Rate Limit Not Enforced

**What goes wrong:** Too many concurrent index updates cause system slowdown

**Why it happens:** Events arrive faster than processing capacity

**How to avoid:** Implement queue with explicit rate limiting

**Warning signs:** UI freezes, high CPU usage during active file changes

### Pitfall 4: Missing Incremental Update Logic

**What goes wrong:** Full index rebuild on each change instead of incremental

**Why it happens:** Not using existing `append_to_workspace_index` function

**How to avoid:** Ensure Rust watcher calls existing incremental update functions

**Warning signs:** Very slow updates, high I/O during monitoring

---

## Code Examples

### Existing: Rust File Watch Command

```rust
// commands/watch.rs:22-58
#[command]
pub async fn start_watch(
    app: AppHandle,
    workspaceId: String,
    path: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    // Creates watcher using notify::recommended_watcher
    // Emits file-changed events to Flutter
    // Handles Create/Modify/Remove events
}
```

Source: `log-analyzer/src-tauri/src/commands/watch.rs`

### Existing: Flutter Bridge Service

```dart
// bridge_service.dart:385-405
Future<bool> startWatch({
  required String workspaceId,
  required List<String> paths,
  required bool recursive,
}) async {
  final result = ffi.startWatch(
    workspaceId: workspaceId,
    paths: paths,
    recursive: recursive,
  );
  return result.ok;
}
```

Source: `log-analyzer_flutter/lib/shared/services/bridge_service.dart`

### Existing: Event Handling

```dart
// event_stream_service.dart:425-437
void _handleFileChanged(dynamic data) {
  if (data is! Map<String, dynamic>) return;
  final eventData = data['event'] as Map<String, dynamic>?;
  if (eventData == null) return;

  final event = FileChangeEvent.fromJson(eventData);
  emitFileChanged(event);
}
```

Source: `log-analyzer_flutter/lib/shared/services/event_stream_service.dart`

---

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| Polling for changes | Event-driven (notify crate) | Existing | Real-time updates |
| Full index rebuild | Incremental updates | Existing | Faster updates |
| No UI for monitoring | Toolbar + status panel | This phase | User control |

**Deprecated/outdated:**
- Manual file polling (replaced by notify crate events)

---

## Open Questions

1. **How to connect Tauri events to Flutter?**
   - What we know: FFI bridge exists, EventStreamService has handlers
   - What's unclear: Whether events actually flow from Rust to Flutter
   - Recommendation: Verify event flow during implementation, add HTTP fallback if needed

2. **Queue max length value**
   - What we know: User said "limit max length, drop old requests when exceeded"
   - What's unclear: Specific number - user left to Claude's discretion
   - Recommendation: Use 1000 as default (reasonable for most use cases)

3. **Retry interval timing**
   - What we know: "Retry 3 times, skip after"
   - What's unclear: Time between retries
   - Recommendation: Use exponential backoff starting at 100ms (100ms, 200ms, 400ms)

---

## Validation Architecture

### Test Framework
| Property | Value |
|----------|-------|
| Framework | flutter_test (built-in) + integration_test |
| Config file | `log-analyzer_flutter/test/` directory |
| Quick run command | `flutter test test/features/realtime_monitoring/` |
| Full suite command | `flutter test` |

### Phase Requirements → Test Map
| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| MON-01 | Enable monitoring via toolbar button | Widget | `flutter test test/monitoring_toolbar_test.dart` | Need to create |
| MON-01 | Disable monitoring via toolbar button | Widget | `flutter test test/monitoring_toolbar_test.dart` | Need to create |
| MON-02 | Index updates on file change | Integration | `flutter test test/file_change_test.dart` | Need to create |
| MON-02 | Deleted files removed from index | Integration | Need to create |
| MON-03 | Status panel shows event count | Widget | Need to create |
| MON-03 | Status panel shows pending count | Widget | Need to create |

### Sampling Rate
- **Per task commit:** `flutter test test/features/realtime_monitoring/ -x`
- **Per wave merge:** `flutter test`
- **Phase gate:** Full suite green before `/gsd:verify-work`

### Wave 0 Gaps
- [ ] `test/features/realtime_monitoring/monitoring_toolbar_test.dart` - covers MON-01
- [ ] `test/features/realtime_monitoring/file_change_test.dart` - covers MON-02
- [ ] `test/features/realtime_monitoring/status_panel_test.dart` - covers MON-03
- [ ] `test/features/realtime_monitoring/monitoring_provider_test.dart` - state management
- [ ] Framework install: Flutter SDK already in project

---

## Sources

### Primary (HIGH confidence)
- Context7: `notify` crate - file system watching API and examples
- Project code: `log-analyzer/src-tauri/src/commands/watch.rs` - existing watch implementation
- Project code: `log-analyzer_flutter/lib/shared/services/bridge_service.dart` - FFI bridge methods

### Secondary (MEDIUM confidence)
- WebSearch: "flutter riverpod file watching best practices" - verified with official Riverpod docs
- WebSearch: "tauri event emission to flutter" - cross-referenced with Tauri 2.0 docs

### Tertiary (LOW confidence)
- None needed - existing project infrastructure well understood

---

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH - Existing infrastructure confirmed in codebase
- Architecture: HIGH - Clear separation between Rust backend and Flutter frontend
- Pitfalls: MEDIUM - Based on common Flutter/Rust integration patterns

**Research date:** 2026-03-03
**Valid until:** 2026-04-03 (30 days for stable domain)

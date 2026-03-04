# Phase 8: 状态管理 - Research

**Researched:** 2026-03-05
**Domain:** Flutter 状态管理 + Riverpod 3.0 + flutter_fancy_tree_view2
**Confidence:** HIGH

## Summary

本阶段研究如何使用 Riverpod 3.0 实现搜索历史和虚拟文件树的状态管理，为 Phase 9（高级搜索 UI）和 Phase 10（虚拟文件系统 UI）提供响应式数据源。

**Primary recommendation:** 使用 Riverpod 3.0 的 `@riverpod` 注解进行代码生成，结合 AsyncNotifierProvider 处理异步 FFI 调用，使用 flutter_fancy_tree_view2 的 TreeController 管理文件树展开状态。

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions

#### Provider 结构设计
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

#### 数据同步策略
- **同步时机**: 自动同步 — 搜索完成后自动保存到历史，文件树切换工作区时自动加载
- **更新模式**: 乐观更新 — 先更新 UI，后同步后端，失败时回滚
- **冲突解决**: 后端数据优先 — 乐观更新失败时以数据库为准，前端同步后端数据
- **刷新触发**: 搜索完成后 + 文件导入后 + 手动刷新按钮 + 文件监控事件

#### 缓存与内存管理
- **内存缓存**: Provider 自动缓存 — Riverpod Provider 自动管理缓存，窗口关闭时自动释放
- **缓存失效**: 工作区切换时失效 — 切换工作区时自动失效缓存，重新加载新工作区数据
- **文件内容缓存**: 只缓存结构 — Provider 只缓存文件树结构，文件内容按需从后端读取，不缓存
- **懒加载策略**: 实时懒加载 — 展开目录时实时从后端加载子节点，数据最新

#### 错误处理模式
- **错误传播**: AsyncError — Provider 捕获错误并转换为 AsyncError，UI 层根据 hasError 显示错误状态
- **重试策略**: 自动重试 — 使用 Riverpod 的 retry 策略，自动重试 3 次
- **加载状态**: AsyncLoading — 使用 Riverpod 的 AsyncLoading 状态，UI 显示加载指示器
- **空状态处理**: UI 层处理 — Provider 返回空列表，UI 层负责显示空状态界面

### Claude's Discretion

None — 所有决策已在 CONTEXT.md 中锁定

### Deferred Ideas (OUT OF SCOPE)

None — discussion stayed within phase scope
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|-----------------|
| HIST-01 | 搜索自动保存到搜索历史 | SearchHistoryProvider.addSearchHistory() + FFI addSearchHistory |
| HIST-02 | 用户可以在下拉列表中查看历史搜索记录 | SearchHistoryProvider 返回 AsyncValue<List<SearchHistoryData>> |
| HIST-03 | 用户可以点击历史记录快速填充搜索框 | SearchHistoryProvider 提供查询数据，UI 处理填充 |
| HIST-04 | 用户可以删除单条历史记录 | SearchHistoryProvider.deleteSearchHistory() |
| HIST-05 | 用户可以清空所有搜索历史 | SearchHistoryProvider.clearSearchHistory() |
| VFS-01 | 用户可以查看工作区的虚拟文件树结构 | VirtualFileTreeProvider.getVirtualFileTree() |
| VFS-02 | 目录节点可以展开/折叠 | flutter_fancy_tree_view2 TreeController |
| VFS-03 | 用户可以点击文件预览内容 | BridgeService.readFileByHash() |
| VFS-04 | 文件树显示文件/目录图标区分 | VirtualTreeNodeData 枚举类型 (File/Archive) |
</phase_requirements>

## Standard Stack

### Core
| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| flutter_riverpod | ^3.0.0 | 状态管理核心 | Flutter Favorite，声明式编程，内置异步支持 |
| riverpod_annotation | ^3.0.0 | 代码生成注解 | 类型安全，减少样板代码，编译时检查 |
| hooks_riverpod | ^3.0.0 | Flutter Hooks 集成 | 与 flutter_hooks 配合，更简洁的 Widget |
| riverpod_generator | ^3.0.0 | 代码生成器 | 自动生成 Provider 代码，减少手动维护 |
| freezed | ^3.2.3 | 不可变数据类 | Riverpod 官方推荐，copyWith 支持，模式匹配 |
| freezed_annotation | ^3.0.0 | Freezed 注解 | 配合 build_runner 使用 |

### Supporting
| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| flutter_fancy_tree_view2 | ^1.6.3 | 树形视图组件 | 虚拟文件树展示，支持懒加载 |
| build_runner | ^2.4.0 | 代码生成运行器 | 运行 `dart run build_runner build` |
| json_serializable | ^6.11.2 | JSON 序列化 | Freezed 类需要 JSON 支持时 |

### Alternatives Considered
| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| flutter_fancy_tree_view2 | two_dimensional_scrollables | Flutter 3.24+ 官方方案，但 API 不够成熟 |
| Riverpod | Provider/Bloc | Provider 已过时，Bloc 学习曲线陡峭 |
| Freezed | built_value | Freezed 更简洁，Riverpod 官方推荐 |

**安装:**
```bash
# 已在 pubspec.yaml 中配置
flutter pub get
```

## Architecture Patterns

### Recommended Project Structure
```
lib/
├── shared/
│   ├── providers/
│   │   ├── search_history_provider.dart      # 搜索历史 Provider
│   │   ├── search_history_provider.g.dart    # 生成的代码
│   │   ├── virtual_file_tree_provider.dart   # 虚拟文件树 Provider
│   │   └── virtual_file_tree_provider.g.dart # 生成的代码
│   ├── models/
│   │   ├── search_history.dart               # 搜索历史模型 (Freezed)
│   │   ├── search_history.freezed.dart       # 生成的 Freezed 代码
│   │   └── search_history.g.dart             # 生成的 JSON 代码
│   └── services/
│       └── bridge_service.dart               # FFI 服务 (已存在)
```

### Pattern 1: AsyncNotifierProvider with Parameter
**What:** 使用 Family 参数化 Provider，支持工作区切换
**When to use:** 需要根据 workspaceId 加载不同数据时
**Example:**
```dart
// Source: 现有 workspace_provider.dart 模式 + Riverpod 3.0 官方文档
import 'package:riverpod_annotation/riverpod_annotation.dart';

part 'search_history_provider.g.dart';

@riverpod
class SearchHistory extends _$SearchHistory {
  @override
  Future<List<SearchHistoryData>> build(String workspaceId) async {
    // 懒加载：首次 watch 时调用
    final bridge = ref.watch(bridgeServiceProvider);
    return bridge.getSearchHistory(workspaceId: workspaceId);
  }

  /// 添加搜索历史
  Future<void> addSearchHistory({
    required String query,
    required int resultCount,
  }) async {
    final bridge = ref.read(bridgeServiceProvider);

    // 乐观更新
    final newItem = SearchHistoryData(
      query: query,
      workspaceId: workspaceId,
      resultCount: resultCount,
      searchedAt: DateTime.now().toIso8601String(),
    );
    state = AsyncData([...state.value ?? [], newItem]);

    // 后端同步
    try {
      await bridge.addSearchHistory(
        query: query,
        workspaceId: workspaceId,
        resultCount: resultCount,
      );
    } catch (e) {
      // 回滚：重新从后端加载
      state = await AsyncValue.guard(() =>
        bridge.getSearchHistory(workspaceId: workspaceId)
      );
      rethrow;
    }
  }

  /// 删除单条历史
  Future<void> deleteSearchHistory(String query) async {
    final bridge = ref.read(bridgeServiceProvider);
    final previous = state.value ?? [];

    // 乐观更新
    state = AsyncData(
      previous.where((h) => h.query != query).toList()
    );

    try {
      await bridge.deleteSearchHistory(query: query, workspaceId: workspaceId);
    } catch (e) {
      state = AsyncData(previous);
      rethrow;
    }
  }

  /// 清空历史
  Future<void> clearSearchHistory() async {
    final bridge = ref.read(bridgeServiceProvider);
    state = const AsyncData([]);
    await bridge.clearSearchHistory(workspaceId: workspaceId);
  }
}
```

### Pattern 2: TreeController Integration
**What:** 使用 flutter_fancy_tree_view2 的 TreeController 管理展开状态
**When to use:** 虚拟文件树需要懒加载子节点
**Example:**
```dart
// Source: flutter_fancy_tree_view2 pub.dev 文档
import 'package:flutter_fancy_tree_view/flutter_fancy_tree_view.dart';

class VirtualFileTreeController extends TreeController<VirtualTreeNodeData> {
  final String workspaceId;
  final BridgeService bridgeService;

  VirtualFileTreeController({
    required this.workspaceId,
    required this.bridgeService,
    required super.roots,
  }) : super(
    childrenProvider: (node) => _getChildren(node),
  );

  static List<VirtualTreeNodeData> _getChildren(VirtualTreeNodeData node) {
    // Archive 节点有 children，File 节点没有
    return switch (node) {
      VirtualTreeNodeDataArchive(:final children) => children,
      VirtualTreeNodeDataFile() => [],
    };
  }

  /// 懒加载子节点（重写展开逻辑）
  @override
  Future<void> toggleExpansion(VirtualTreeNodeData node) async {
    if (node is VirtualTreeNodeDataArchive &&
        (node.children.isEmpty)) {
      // 从后端加载子节点
      final children = await bridgeService.getTreeChildren(
        workspaceId: workspaceId,
        parentPath: node.path,
      );

      // 更新节点的 children
      // 注意：由于 Freezed 不可变，需要重新构建树
      // 实际实现中应使用状态管理
    }
    super.toggleExpansion(node);
  }
}
```

### Anti-Patterns to Avoid
- **在 build() 中直接调用异步方法**: 使用 `Future.microtask()` 延迟执行，或在 `build()` 中返回初始值
- **忽略 AsyncValue 的 loading/error 状态**: UI 必须处理所有三种状态
- **手动管理缓存**: Riverpod 自动管理，无需手动缓存
- **在 Provider 中存储可变状态**: 必须使用 Freezed 不可变类

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| 状态管理 | 手写 StatefulWidget + setState | Riverpod | 自动缓存、依赖追踪、异步支持 |
| 树展开状态 | 手写 Map<Node, bool> | TreeController | 内置展开/折叠管理，动画支持 |
| 不可变数据 | 手写 copyWith | Freezed | 自动生成，类型安全，模式匹配 |
| 异步加载状态 | 手写 isLoading/isError 变量 | AsyncValue | 内置 loading/data/error 状态 |
| LRU 缓存 | 手写 LinkedHashMap | 后端 FFI | 后端已实现，前端无需关心 |

**Key insight:** Riverpod 3.0 的 AsyncValue 和代码生成极大减少了状态管理的样板代码，配合 Freezed 实现完全不可变的状态模型。

## Common Pitfalls

### Pitfall 1: Provider 参数变化时状态丢失
**What goes wrong:** 切换工作区时，SearchHistoryProvider 不会自动刷新
**Why it happens:** Riverpod 默认缓存 Provider 状态
**How to avoid:** 使用 Family 参数化，参数变化时自动重新 build
```dart
// 正确：使用参数化
@riverpod
class SearchHistory extends _$SearchHistory {
  @override
  Future<List<SearchHistoryData>> build(String workspaceId) async {
    // workspaceId 变化时自动重新执行
  }
}

// UI 中使用
final history = ref.watch(searchHistoryProvider(workspaceId));
```
**Warning signs:** 切换工作区后数据未更新

### Pitfall 2: TreeController 与 Provider 状态不同步
**What goes wrong:** 文件树展开状态与数据源不一致
**Why it happens:** TreeController 内部管理展开状态，不与 Riverpod 同步
**How to avoid:** 将 TreeController 作为 Provider 状态的一部分
```dart
@riverpod
class VirtualFileTree extends _$VirtualFileTree {
  @override
  Future<TreeController<VirtualTreeNodeData>> build(String workspaceId) async {
    final bridge = ref.watch(bridgeServiceProvider);
    final roots = await bridge.getVirtualFileTree(workspaceId);

    return VirtualFileTreeController(
      workspaceId: workspaceId,
      bridgeService: bridge,
      roots: roots,
    );
  }
}
```
**Warning signs:** 展开/折叠状态在切换页面后丢失

### Pitfall 3: 乐观更新失败后状态不一致
**What goes wrong:** 删除历史后网络失败，UI 显示已删除但后端未删除
**Why it happens:** 乐观更新没有正确回滚
**How to avoid:** 保存之前状态，失败时恢复
```dart
Future<void> deleteSearchHistory(String query) async {
  final previous = state.value ?? [];

  // 乐观更新
  state = AsyncData(previous.where((h) => h.query != query).toList());

  try {
    await bridge.deleteSearchHistory(query: query, workspaceId: workspaceId);
  } catch (e) {
    // 回滚到之前状态
    state = AsyncData(previous);
    rethrow;
  }
}
```
**Warning signs:** 操作失败后 UI 与后端数据不一致

## Code Examples

Verified patterns from official sources:

### AsyncNotifierProvider with Family
```dart
// Source: Riverpod 3.0 官方文档 - https://riverpod.dev
@riverpod
Future<String> boredSuggestion(Ref ref) async {
  final response = await http.get(
    Uri.https('boredapi.com', '/api/activity'),
  );
  final json = jsonDecode(response.body) as Map;
  return json['activity']! as String;
}

// UI 中使用
class Home extends ConsumerWidget {
  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final boredSuggestion = ref.watch(boredSuggestionProvider);
    return switch (boredSuggestion) {
      AsyncData(:final value) => Text('data: $value'),
      AsyncError(:final error) => Text('error: $error'),
      _ => const Text('loading'),
    };
  }
}
```

### Freezed 不可变类
```dart
// Source: Freezed 官方文档
import 'package:freezed_annotation/freezed_annotation.dart';

part 'search_history.freezed.dart';
part 'search_history.g.dart';

@freezed
class SearchHistoryModel with _$SearchHistoryModel {
  const factory SearchHistoryModel({
    required String query,
    required String workspaceId,
    required int resultCount,
    required DateTime searchedAt,
  }) = _SearchHistoryModel;

  factory SearchHistoryModel.fromJson(Map<String, dynamic> json) =>
      _$SearchHistoryModelFromJson(json);
}
```

### TreeView 懒加载
```dart
// Source: flutter_fancy_tree_view2 pub.dev 文档
class MyTreeNode {
  const MyTreeNode({
    required this.title,
    this.children = const <MyTreeNode>[],
  });
  final String title;
  final List<MyTreeNode> children;
}

final treeController = TreeController<MyTreeNode>(
  roots: roots,
  childrenProvider: (MyTreeNode node) => node.children,
);

@override
Widget build(BuildContext context) {
  return AnimatedTreeView<MyTreeNode>(
    treeController: treeController,
    nodeBuilder: (BuildContext context, TreeEntry<MyTreeNode> entry) {
      return InkWell(
        onTap: () => treeController.toggleExpansion(entry.node),
        child: TreeIndentation(
          entry: entry,
          child: Text(entry.node.title),
        ),
      );
    },
  );
}
```

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| Provider + ChangeNotifier | Riverpod AsyncNotifier | Riverpod 2.0+ | 内置异步支持，无需手动管理 loading 状态 |
| 手写 copyWith | Freezed 代码生成 | Freezed 2.0+ | 减少样板代码，类型安全 |
| flutter_fancy_tree_view | flutter_fancy_tree_view2 | 2024 | 继续维护的 fork，兼容性更好 |
| Tauri invoke | FFI Bridge | Phase 07 | 同步调用，更低延迟 |

**Deprecated/outdated:**
- Provider 包: 已被 Riverpod 取代，Flutter 官方不再推荐
- flutter_fancy_tree_view (原版): 已 discontinued，使用 flutter_fancy_tree_view2

## Open Questions

1. **TreeController 与 Riverpod 状态同步**
   - What we know: TreeController 内部管理展开状态
   - What's unclear: 最佳的同步方式是什么
   - Recommendation: 将 TreeController 作为 Provider 状态的一部分，或使用 StateProvider 存储展开节点 Set

2. **FFI 类型与 Dart 模型转换**
   - What we know: FFI 生成 SearchHistoryData、VirtualTreeNodeData 类型
   - What's unclear: 是否需要额外的 Dart 模型层
   - Recommendation: 直接使用 FFI 生成的类型，避免额外转换开销

## Validation Architecture

### Test Framework
| Property | Value |
|----------|-------|
| Framework | flutter_test (SDK 内置) |
| Config file | flutter_test_config.dart (如需要) |
| Quick run command | `flutter test test/shared/providers/` |
| Full suite command | `flutter test` |

### Phase Requirements → Test Map
| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| HIST-01 | 搜索自动保存 | unit | `flutter test test/shared/providers/search_history_provider_test.dart` | Wave 0 |
| HIST-02 | 查看历史列表 | unit | `flutter test test/shared/providers/search_history_provider_test.dart` | Wave 0 |
| HIST-03 | 点击填充搜索框 | widget | `flutter test test/features/search/widgets/search_bar_test.dart` | Wave 0 |
| HIST-04 | 删除单条历史 | unit | `flutter test test/shared/providers/search_history_provider_test.dart` | Wave 0 |
| HIST-05 | 清空历史 | unit | `flutter test test/shared/providers/search_history_provider_test.dart` | Wave 0 |
| VFS-01 | 查看文件树结构 | unit | `flutter test test/shared/providers/virtual_file_tree_provider_test.dart` | Wave 0 |
| VFS-02 | 目录展开/折叠 | unit | `flutter test test/shared/providers/virtual_file_tree_provider_test.dart` | Wave 0 |
| VFS-03 | 点击预览内容 | widget | `flutter test test/features/vfs/widgets/file_preview_test.dart` | Wave 0 |
| VFS-04 | 文件/目录图标区分 | widget | `flutter test test/features/vfs/widgets/tree_node_test.dart` | Wave 0 |

### Sampling Rate
- **Per task commit:** `flutter test test/shared/providers/`
- **Per wave merge:** `flutter test`
- **Phase gate:** Full suite green before `/gsd:verify-work`

### Wave 0 Gaps
- [ ] `test/shared/providers/search_history_provider_test.dart` — covers HIST-01~05
- [ ] `test/shared/providers/virtual_file_tree_provider_test.dart` — covers VFS-01~02
- [ ] `test/features/search/widgets/search_bar_test.dart` — covers HIST-03
- [ ] `test/features/vfs/widgets/file_preview_test.dart` — covers VFS-03
- [ ] `test/features/vfs/widgets/tree_node_test.dart` — covers VFS-04

## Sources

### Primary (HIGH confidence)
- Riverpod 官方文档 - https://riverpod.dev (2026-03-05 访问)
- flutter_fancy_tree_view2 pub.dev - https://pub.dev/packages/flutter_fancy_tree_view2 (2026-03-05 访问)
- 现有项目代码 - app_provider.dart, workspace_provider.dart 模式参考

### Secondary (MEDIUM confidence)
- Freezed GitHub README - https://github.com/rrousselGit/freezed
- Flutter Rust Bridge 文档 - 已有 bridge_service.dart 实现

### Tertiary (LOW confidence)
- None

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH - Riverpod/Freezed 是 Flutter 官方推荐方案，已在项目中使用
- Architecture: HIGH - 现有代码已建立清晰的 Provider 模式
- Pitfalls: HIGH - 基于 Riverpod 官方文档和现有代码经验

**Research date:** 2026-03-05
**Valid until:** 2026-04-05 (1 个月，Flutter 生态稳定)

---

*Phase: 08-state-management*
*Research completed: 2026-03-05*

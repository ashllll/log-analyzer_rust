# Phase 9: 高级搜索 UI - Research

**Researched:** 2026-03-06
**Domain:** Flutter UI 组件开发、高级搜索功能、Riverpod 3.0 状态管理
**Confidence:** HIGH

## Summary

Phase 9 需要实现高级搜索 UI，包括正则表达式搜索模式、多关键词组合搜索（AND/OR/NOT）、以及搜索历史管理功能。关键发现：

1. **后端 API 已就绪** - Phase 7 已完成所有 FFI 桥接：`validateRegex`、`searchRegex`、`searchStructured`、`buildSearchQuery`、以及完整的搜索历史 CRUD API。
2. **状态管理已就绪** - Phase 8 已完成 `SearchHistoryProvider`，支持乐观更新、参数化工作区、自动刷新。
3. **UI 模式已建立** - 项目使用 Riverpod 3.0 + riverpod_annotation、Material Design、AppTheme 统一样式、CustomInput 等共享组件。

**Primary recommendation:** 在现有 SearchPage 基础上扩展 SearchInputBar 组件，添加搜索模式切换、关键词组合 UI、搜索历史下拉面板，复用已就绪的 Provider 和 FFI API。

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|-----------------|
| ASEARCH-01 | 用户可以切换到正则表达式搜索模式 | SearchModeSelector 组件 + SearchMode enum |
| ASEARCH-02 | 正则表达式搜索时提供语法反馈 | FFI `validateRegex` API + 实时验证 |
| ASEARCH-03 | 用户可以输入多个关键词并选择 AND 组合 | MultiKeywordInput 组件 + `searchStructured` API |
| ASEARCH-04 | 用户可以输入多个关键词并选择 OR 组合 | MultiKeywordInput 组件 + QueryOperator.or |
| ASEARCH-05 | 用户可以输入多个关键词并选择 NOT 组合 | MultiKeywordInput 组件 + QueryOperator.not |
| ASEARCH-06 | 用户可以查看组合后的搜索条件预览 | SearchConditionPreview 组件 |
| HIST-01 | 搜索自动保存到搜索历史 | SearchHistoryProvider.addSearchHistory |
| HIST-02 | 用户可以在下拉列表中查看历史搜索记录 | SearchHistoryDropdown 组件 |
| HIST-03 | 用户可以点击历史记录快速填充搜索框 | SearchHistoryItem.onTap 回调 |
| HIST-04 | 用户可以删除单条历史记录 | SearchHistoryProvider.deleteSearchHistory |
| HIST-05 | 用户可以清空所有搜索历史 | SearchHistoryProvider.clearSearchHistory |
</phase_requirements>

## Standard Stack

### Core
| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| flutter_riverpod | ^3.0.0 | 状态管理 | 项目标准，Phase 8 已使用 |
| riverpod_annotation | ^3.0.0 | Provider 代码生成 | 配合 riverpod_generator |
| freezed | ^3.2.3 | 不可变数据模型 | 项目标准，SearchHistoryItem 模式 |
| flutter_rust_bridge | ^2.0.0 | FFI 桥接 | Phase 7 已集成 |
| material_design | SDK | UI 组件 | Flutter 标准 |

### Supporting
| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| lucide_icons_flutter | ^1.0.0 | 图标库 | 按钮和 UI 图标 |
| uuid | ^4.0.0 | 唯一 ID 生成 | SearchTermData.id |
| collection | ^1.18.0 | 集合操作 | 关键词列表处理 |

### Alternatives Considered
| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| 自定义正则验证 | Dart RegExp | FFI 已实现，统一后端验证逻辑 |
| 本地历史存储 | shared_preferences | FFI 已实现，支持跨工作区 |

**Installation:** (无需新增依赖，使用现有配置)

## Architecture Patterns

### Recommended Project Structure
```
lib/features/search/
├── presentation/
│   ├── search_page.dart          # 主搜索页面（已存在）
│   └── widgets/
│       ├── search_mode_selector.dart   # 搜索模式切换 (NEW)
│       ├── regex_input_field.dart      # 正则输入框+验证 (NEW)
│       ├── multi_keyword_input.dart    # 多关键词输入 (NEW)
│       ├── search_condition_preview.dart # 条件预览 (NEW)
│       └── search_history_dropdown.dart # 历史下拉 (NEW)
├── models/
│   └── search_mode.dart          # 搜索模式枚举 (NEW)
└── providers/
    └── search_query_provider.dart  # 搜索查询状态 (NEW)

lib/shared/
├── providers/
│   └── search_history_provider.dart  # 已存在 (Phase 8)
└── services/
    └── generated/ffi/bridge.dart     # FFI API (Phase 7)
```

### Pattern 1: SearchMode 枚举 + 模式切换
**What:** 使用 Dart 3 sealed class 或 enum 定义搜索模式，通过 ToggleButtons/SegmentedButton 切换
**When to use:** 搜索模式切换（普通/正则/多关键词组合）
**Example:**
```dart
// 搜索模式枚举
enum SearchMode {
  normal,    // 普通搜索
  regex,     // 正则表达式
  combined,  // 多关键词组合
}

// 模式切换组件
class SearchModeSelector extends StatelessWidget {
  final SearchMode currentMode;
  final ValueChanged<SearchMode> onModeChanged;

  @override
  Widget build(BuildContext context) {
    return SegmentedButton<SearchMode>(
      segments: const [
        ButtonSegment(value: SearchMode.normal, label: Text('普通')),
        ButtonSegment(value: SearchMode.regex, label: Text('正则')),
        ButtonSegment(value: SearchMode.combined, label: Text('组合')),
      ],
      selected: {currentMode},
      onSelectionChanged: (Set<SearchMode> selected) {
        onModeChanged(selected.first);
      },
    );
  }
}
```

### Pattern 2: 正则验证 + 实时反馈
**What:** 使用 FFI `validateRegex` API 实时验证正则语法，显示有效/无效状态
**When to use:** 正则表达式输入框
**Example:**
```dart
// 正则输入框 + 实时验证
class RegexInputField extends StatefulWidget {
  final ValueChanged<String>? onChanged;
  final ValueChanged<String>? onSubmitted;

  @override
  State<RegexInputField> createState() => _RegexInputFieldState();
}

class _RegexInputFieldState extends State<RegexInputField> {
  final _controller = TextEditingController();
  RegexValidationResult? _validationResult;
  Timer? _debounceTimer;

  @override
  void initState() {
    super.initState();
    _controller.addListener(_onTextChanged);
  }

  void _onTextChanged() {
    _debounceTimer?.cancel();
    _debounceTimer = Timer(const Duration(milliseconds: 300), _validateRegex);
  }

  Future<void> _validateRegex() async {
    final pattern = _controller.text;
    if (pattern.isEmpty) {
      setState(() => _validationResult = null);
      return;
    }

    // 使用 FFI 验证
    final result = ffi.validateRegex(pattern: pattern);
    setState(() => _validationResult = result);
  }

  @override
  Widget build(BuildContext context) {
    return TextField(
      controller: _controller,
      decoration: InputDecoration(
        hintText: '输入正则表达式...',
        suffixIcon: _buildValidationIcon(),
        errorText: _validationResult?.valid == false
            ? _validationResult!.errorMessage
            : null,
        helperText: _validationResult?.valid == true ? '语法有效' : null,
      ),
      onSubmitted: widget.onSubmitted,
    );
  }

  Widget? _buildValidationIcon() {
    if (_validationResult == null) return null;
    return Icon(
      _validationResult!.valid ? Icons.check_circle : Icons.error,
      color: _validationResult!.valid ? AppColors.success : AppColors.error,
    );
  }
}
```

### Pattern 3: 多关键词组合 UI
**What:** 使用 Chip 组件显示关键词列表，提供 AND/OR/NOT 操作符选择
**When to use:** 多关键词组合搜索
**Example:**
```dart
// 关键词输入 + 操作符选择
class MultiKeywordInput extends StatefulWidget {
  final List<SearchTermData> terms;
  final QueryOperatorData globalOperator;
  final ValueChanged<List<SearchTermData>> onTermsChanged;
  final ValueChanged<QueryOperatorData> onOperatorChanged;

  @override
  State<MultiKeywordInput> createState() => _MultiKeywordInputState();
}

class _MultiKeywordInputState extends State<MultiKeywordInput> {
  final _inputController = TextEditingController();

  void _addKeyword() {
    final value = _inputController.text.trim();
    if (value.isEmpty) return;

    final newTerm = SearchTermData(
      id: const Uuid().v4(),
      value: value,
      operator_: QueryOperatorData.and, // 默认 AND
      isRegex: false,
      priority: widget.terms.length,
      enabled: true,
      caseSensitive: false,
    );

    widget.onTermsChanged([...widget.terms, newTerm]);
    _inputController.clear();
  }

  void _removeKeyword(String id) {
    widget.onTermsChanged(
      widget.terms.where((t) => t.id != id).toList(),
    );
  }

  @override
  Widget build(BuildContext context) {
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        // 操作符选择
        Row(
          children: [
            const Text('组合方式: '),
            SegmentedButton<QueryOperatorData>(
              segments: const [
                ButtonSegment(value: QueryOperatorData.and, label: Text('AND')),
                ButtonSegment(value: QueryOperatorData.or, label: Text('OR')),
                ButtonSegment(value: QueryOperatorData.not, label: Text('NOT')),
              ],
              selected: {widget.globalOperator},
              onSelectionChanged: (selected) =>
                  widget.onOperatorChanged(selected.first),
            ),
          ],
        ),
        const SizedBox(height: 8),
        // 已添加的关键词
        Wrap(
          spacing: 8,
          runSpacing: 4,
          children: widget.terms.map((term) => Chip(
            label: Text(term.value),
            onDeleted: () => _removeKeyword(term.id),
          )).toList(),
        ),
        // 输入框
        TextField(
          controller: _inputController,
          decoration: const InputDecoration(
            hintText: '输入关键词后按 Enter 添加',
          ),
          onSubmitted: (_) => _addKeyword(),
        ),
      ],
    );
  }
}
```

### Pattern 4: 搜索历史下拉面板
**What:** 使用 PopupMenuButton 或自定义 Overlay 显示搜索历史列表
**When to use:** 搜索历史快速访问
**Example:**
```dart
// 搜索历史下拉面板
class SearchHistoryDropdown extends ConsumerWidget {
  final String workspaceId;
  final ValueChanged<String> onSelect;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final historyAsync = ref.watch(searchHistoryProvider(workspaceId));

    return historyAsync.when(
      data: (history) {
        if (history.isEmpty) {
          return const SizedBox.shrink();
        }

        return PopupMenuButton<String>(
          icon: const Icon(Icons.history),
          tooltip: '搜索历史',
          itemBuilder: (context) => history.map((item) {
            return PopupMenuItem<String>(
              value: item.query,
              child: Row(
                children: [
                  const Icon(Icons.history, size: 16),
                  const SizedBox(width: 8),
                  Expanded(child: Text(item.query)),
                  Text(
                    '${item.resultCount} 条结果',
                    style: Theme.of(context).textTheme.bodySmall,
                  ),
                  // 删除按钮
                  IconButton(
                    icon: const Icon(Icons.close, size: 16),
                    onPressed: () {
                      ref.read(searchHistoryProvider(workspaceId).notifier)
                          .deleteSearchHistory(item.query);
                    },
                  ),
                ],
              ),
            );
          }).toList(),
          onSelected: onSelect,
        );
      },
      loading: () => const CircularProgressIndicator(),
      error: (_, __) => const SizedBox.shrink(),
    );
  }
}
```

### Anti-Patterns to Avoid
- **直接使用 RegExp 类验证** - 应使用 FFI `validateRegex` 保持与后端一致
- **在 build() 中调用异步方法** - 应使用 `Future.microtask` 或 `ref.listen` 模式
- **忽略乐观更新** - 搜索历史操作应先更新 UI，失败时回滚
- **硬编码颜色** - 应使用 AppColors 统一主题

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| 正则验证 | Dart RegExp | FFI `validateRegex` | 与后端一致，支持所有 Rust 正则特性 |
| 搜索历史存储 | SharedPreferences | FFI `addSearchHistory` | 已实现，支持跨工作区 |
| 结构化搜索 | 手动拼接查询 | FFI `buildSearchQuery` | 已实现，类型安全 |
| 状态管理 | setState | Riverpod AsyncNotifier | Phase 8 已建立模式 |

**Key insight:** Phase 7-8 已完成所有后端集成，本阶段专注于 UI 层实现，复用现有 FFI API 和 Provider。

## Common Pitfalls

### Pitfall 1: 正则验证不同步
**What goes wrong:** Dart RegExp 和 Rust regex crate 语法不完全一致，导致前端验证通过但后端执行失败
**Why it happens:** 两个正则引擎语法差异
**How to avoid:** 必须使用 FFI `validateRegex` API 进行验证
**Warning signs:** 前端显示"语法有效"但后端搜索报错

### Pitfall 2: 搜索历史不刷新
**What goes wrong:** 添加/删除历史后列表未更新
**Why it happens:** 未正确使用 Riverpod 参数化 Provider
**How to avoid:** 使用 `searchHistoryProvider(workspaceId)` 参数化，乐观更新后自动刷新
**Warning signs:** 执行搜索后历史列表不变

### Pitfall 3: 内存泄漏（Timer 未取消）
**What goes wrong:** 正则验证防抖 Timer 在组件销毁后仍在执行
**Why it happens:** 未在 dispose() 中取消 Timer
**How to avoid:** 在 State dispose() 中调用 `_debounceTimer?.cancel()`
**Warning signs:** 控制台报错 "setState() called after dispose()"

### Pitfall 4: 关键词列表状态混乱
**What goes wrong:** 添加/删除关键词后列表显示异常
**Why it happens:** 直接修改列表而非创建新列表
**How to avoid:** 使用 `[...widget.terms, newTerm]` 创建新列表，保持不可变性
**Warning signs:** 关键词 Chip 位置错乱或重复

## Code Examples

### 完整搜索流程（正则模式）
```dart
// 执行正则搜索
Future<void> _performRegexSearch(String pattern) async {
  final workspaceId = ref.read(appStateProvider).activeWorkspaceId;
  if (workspaceId == null) return;

  setState(() {
    _isSearching = true;
    _logs = [];
  });

  try {
    // 使用 FFI 执行正则搜索
    final results = ffi.searchRegex(
      pattern: pattern,
      workspaceId: workspaceId,
      maxResults: AppConstants.defaultMaxResults,
      caseSensitive: false,
    );

    // 保存到搜索历史
    ref.read(searchHistoryProvider(workspaceId).notifier).addSearchHistory(
      query: pattern,
      resultCount: results.length,
    );

    setState(() {
      _logs = results.map(_convertToLogEntry).toList();
      _isSearching = false;
    });
  } catch (e) {
    setState(() => _isSearching = false);
    ref.read(appStateProvider.notifier).addToast(
      ToastType.error,
      '搜索失败: $e',
    );
  }
}
```

### 完整搜索流程（多关键词组合）
```dart
// 执行结构化搜索
Future<void> _performStructuredSearch(
  List<SearchTermData> terms,
  QueryOperatorData globalOperator,
) async {
  final workspaceId = ref.read(appStateProvider).activeWorkspaceId;
  if (workspaceId == null || terms.isEmpty) return;

  setState(() {
    _isSearching = true;
    _logs = [];
  });

  try {
    // 构建查询对象
    final query = ffi.StructuredSearchQueryData(
      terms: terms,
      globalOperator: globalOperator,
    );

    // 执行结构化搜索
    final results = ffi.searchStructured(
      query: query,
      workspaceId: workspaceId,
      maxResults: AppConstants.defaultMaxResults,
    );

    // 保存到搜索历史（使用预览字符串）
    final queryPreview = _buildQueryPreview(terms, globalOperator);
    ref.read(searchHistoryProvider(workspaceId).notifier).addSearchHistory(
      query: queryPreview,
      resultCount: results.length,
    );

    setState(() {
      _logs = results.map(_convertToLogEntry).toList();
      _isSearching = false;
    });
  } catch (e) {
    setState(() => _isSearching = false);
    ref.read(appStateProvider.notifier).addToast(
      ToastType.error,
      '搜索失败: $e',
    );
  }
}

// 构建查询预览字符串
String _buildQueryPreview(List<SearchTermData> terms, QueryOperatorData op) {
  final opStr = op == QueryOperatorData.and ? ' AND ' :
                op == QueryOperatorData.or ? ' OR ' : ' NOT ';
  return terms.map((t) => t.value).join(opStr);
}
```

### 清空搜索历史（带确认对话框）
```dart
// 清空所有搜索历史
Future<void> _clearAllHistory() async {
  final confirmed = await showDialog<bool>(
    context: context,
    builder: (context) => AlertDialog(
      title: const Text('确认清空'),
      content: const Text('确定要清空所有搜索历史吗？此操作不可恢复。'),
      actions: [
        TextButton(
          onPressed: () => Navigator.pop(context, false),
          child: const Text('取消'),
        ),
        ElevatedButton(
          onPressed: () => Navigator.pop(context, true),
          style: ElevatedButton.styleFrom(
            backgroundColor: AppColors.error,
          ),
          child: const Text('确认清空'),
        ),
      ],
    ),
  );

  if (confirmed == true) {
    final workspaceId = ref.read(appStateProvider).activeWorkspaceId;
    if (workspaceId != null) {
      await ref.read(searchHistoryProvider(workspaceId).notifier)
          .clearSearchHistory();
      ref.read(appStateProvider.notifier).addToast(
        ToastType.success,
        '搜索历史已清空',
      );
    }
  }
}
```

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| 手动正则验证 | FFI `validateRegex` | Phase 7 | 与后端一致 |
| 本地历史存储 | FFI + Riverpod Provider | Phase 8 | 跨工作区同步 |
| setState 状态管理 | Riverpod AsyncNotifier | Phase 8 | 乐观更新 + 自动刷新 |

**Deprecated/outdated:**
- 直接使用 Dart RegExp 类验证: 使用 FFI `validateRegex` 替代
- SharedPreferences 存储历史: 使用 FFI 搜索历史 API 替代

## Open Questions

1. **搜索条件预览样式**
   - What we know: 需要 AND/OR/NOT 组合预览
   - What's unclear: 预览文本格式（`error AND warning` vs `error + warning`）
   - Recommendation: 使用 SQL 风格 `keyword1 AND keyword2`，直观且与后端一致

2. **历史记录数量限制**
   - What we know: FFI `getSearchHistory` 支持 `limit` 参数
   - What's unclear: 默认显示多少条（建议 10-20 条）
   - Recommendation: 默认显示最近 20 条，支持滚动查看更多

## Validation Architecture

### Test Framework
| Property | Value |
|----------|-------|
| Framework | flutter_test (SDK) |
| Config file | analysis_options.yaml |
| Quick run command | `flutter test test/features/search/` |
| Full suite command | `flutter test` |

### Phase Requirements -> Test Map
| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| ASEARCH-01 | 搜索模式切换 | widget | `flutter test test/features/search/widgets/search_mode_selector_test.dart` | Wave 0 |
| ASEARCH-02 | 正则语法验证反馈 | widget | `flutter test test/features/search/widgets/regex_input_field_test.dart` | Wave 0 |
| ASEARCH-03~05 | 多关键词组合 | widget | `flutter test test/features/search/widgets/multi_keyword_input_test.dart` | Wave 0 |
| ASEARCH-06 | 条件预览显示 | widget | `flutter test test/features/search/widgets/search_condition_preview_test.dart` | Wave 0 |
| HIST-01~05 | 搜索历史管理 | unit | `flutter test test/shared/providers/search_history_provider_test.dart` | Partial (Phase 8) |

### Sampling Rate
- **Per task commit:** `flutter test test/features/search/`
- **Per wave merge:** `flutter test`
- **Phase gate:** Full suite green before `/gsd:verify-work`

### Wave 0 Gaps
- [ ] `test/features/search/widgets/search_mode_selector_test.dart` - covers ASEARCH-01
- [ ] `test/features/search/widgets/regex_input_field_test.dart` - covers ASEARCH-02
- [ ] `test/features/search/widgets/multi_keyword_input_test.dart` - covers ASEARCH-03~05
- [ ] `test/features/search/widgets/search_condition_preview_test.dart` - covers ASEARCH-06
- [ ] `test/features/search/widgets/search_history_dropdown_test.dart` - covers HIST-02~04
- [ ] `test/features/search/providers/search_query_provider_test.dart` - covers query state

*(SearchHistoryProvider 测试已在 Phase 8 完成部分覆盖，需补充 UI 集成测试)*

## Sources

### Primary (HIGH confidence)
- FFI Bridge API - `lib/shared/services/generated/ffi/bridge.dart` - 正则验证、结构化搜索 API
- FFI Types - `lib/shared/services/generated/ffi/types.dart` - QueryOperatorData、SearchTermData、StructuredSearchQueryData
- SearchHistoryProvider - `lib/shared/providers/search_history_provider.dart` - 状态管理模式

### Secondary (MEDIUM confidence)
- SearchPage 现有实现 - `lib/features/search/presentation/search_page.dart` - UI 模式参考
- CustomInput 组件 - `lib/shared/widgets/custom_input.dart` - 输入框组件模式
- Provider 测试模式 - `test/shared/providers/app_provider_test.dart` - 测试模式参考

### Tertiary (LOW confidence)
- N/A - 所有信息均来自项目源码

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH - 基于 Phase 7-8 已实现的技术栈
- Architecture: HIGH - 遵循现有 Flutter + Riverpod 模式
- Pitfalls: HIGH - 基于 Flutter 和 Riverpod 常见问题

**Research date:** 2026-03-06
**Valid until:** 30 days (stable Flutter/Riverpod ecosystem)

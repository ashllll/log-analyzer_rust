---
phase: 09-advanced-search-ui
plan: 05
type: gap_closure
wave: 3
depends_on: ["09-02"]
files_modified:
  - log-analyzer_flutter/lib/features/search/presentation/search_page.dart
autonomous: true
requirements:
  - ASEARCH-03
  - ASEARCH-04
  - ASEARCH-05
  - ASEARCH-06
must_haves:
  truths:
    - "用户可以在 SearchPage 的 combined 模式下看到 MultiKeywordInput 组件"
    - "用户可以在搜索按钮上方看到 SearchConditionPreview 显示组合后的条件"
    - "组合搜索功能正常工作"
  artifacts:
    - path: "log-analyzer_flutter/lib/features/search/presentation/search_page.dart"
      provides: "集成组合搜索组件"
---

<objective>
将已存在的 MultiKeywordInput 和 SearchConditionPreview 组件集成到 SearchPage 的 combined 模式中。

Purpose: 完成 ASEARCH-03/04/05/06 的集成工作。
Output: SearchPage 的 combined 模式显示实际的组件而非占位符。
</objective>

<execution_context>
@C:/Users/white/.claude/get-shit-done/workflows/execute-plan.md
</execution_context>

<tasks>

<task type="auto">
  <name>Task 1: 添加缺失的 imports</name>
  <files>
    log-analyzer_flutter/lib/features/search/presentation/search_page.dart
  </files>
  <action>
在 search_page.dart 顶部添加缺失的 imports：

1. **添加 imports:**
```dart
import 'widgets/multi_keyword_input.dart';
import 'widgets/search_condition_preview.dart';
import '../providers/search_query_provider.dart';
```

2. **验证位置:**
   - 在现有 widget imports 之后添加
   - 确保路径正确
  </action>
  <verify>
```bash
cd log-analyzer_flutter && flutter analyze lib/features/search/presentation/search_page.dart
```
  </verify>
  <done>
- imports 添加正确
- 无 analyzer 错误
  </done>
</task>

<task type="auto">
  <name>Task 2: 替换 combined 模式占位符</name>
  <files>
    log-analyzer_flutter/lib/features/search/presentation/search_page.dart
  </files>
  <action>
在 `_buildSearchInput()` 方法中替换 combined 模式的占位符。

1. **定位占位符:**
   - 查找 `case SearchMode.combined:` 分支
   - 当前是显示 "组合搜索（09-02 计划实现）" 的 disabled TextField

2. **替换为实际组件:**
```dart
case SearchMode.combined:
  return MultiKeywordInput(
    terms: ref.watch(searchQueryProvider).terms,
    globalOperator: ref.watch(searchQueryProvider).globalOperator,
    onTermsChanged: (terms) {
      ref.read(searchQueryProvider.notifier).setTerms(terms);
    },
    onOperatorChanged: (op) {
      ref.read(searchQueryProvider.notifier).setGlobalOperator(op);
    },
  );
```

3. **注意:**
   - 使用 ref.watch 监听 provider 状态
   - 使用 ref.read 调用 notifier 方法更新状态
  </action>
  <verify>
```bash
cd log-analyzer_flutter && flutter analyze lib/features/search/presentation/search_page.dart
```
  </verify>
  <done>
- combined 模式显示 MultiKeywordInput
- 输入框可正常输入
- 无 analyzer 错误
  </done>
</task>

<task type="auto">
  <name>Task 3: 添加 SearchConditionPreview</name>
  <files>
    log-analyzer_flutter/lib/features/search/presentation/search_page.dart
  </files>
  <action>
在搜索按钮上方添加 SearchConditionPreview 组件，仅在 combined 模式下显示。

1. **在搜索按钮上方添加条件预览:**
```dart
// 在搜索按钮之前添加
if (_searchMode == SearchMode.combined)
  SearchConditionPreview(
    terms: ref.watch(searchQueryProvider).terms,
    globalOperator: ref.watch(searchQueryProvider).globalOperator,
  ),
const SizedBox(height: 8),
// 搜索按钮...
```

2. **位置建议:**
   - 在 _buildSearchBar() 方法中
   - 搜索按钮之前
   - 仅 combined 模式显示
  </action>
  <verify>
```bash
cd log-analyzer_flutter && flutter analyze lib/features/search/presentation/search_page.dart
```
  </verify>
  <done>
- SearchConditionPreview 在 combined 模式下显示
- 显示格式如 "keyword1 AND keyword2"
- 无 analyzer 错误
  </done>
</task>

<task type="auto">
  <name>Task 4: 最终验证和提交</name>
  <files>
    log-analyzer_flutter/lib/features/search/presentation/search_page.dart
  </files>
  <action>
最终验证和提交更改。

1. **运行完整 analyze:**
```bash
cd log-analyzer_flutter && flutter analyze lib/features/search/
```

2. **提交更改:**
```bash
git add log-analyzer_flutter/lib/features/search/presentation/search_page.dart
git commit -m "fix(09): integrate combined search components into SearchPage

- Replace combined mode placeholder with MultiKeywordInput
- Add SearchConditionPreview for condition preview
- Wire searchQueryProvider for state management

Completes ASEARCH-03, ASEARCH-04, ASEARCH-05, ASEARCH-06 requirements.

Co-Authored-By: Claude Opus 4.6 <noreply@anthropic.com>"
```

3. **更新 VERIFICATION.md:**
   - 将状态从 PARTIAL 改为 PASSED
   - 更新 gap analysis 显示已修复
  </action>
  <verify>
```bash
cd log-analyzer_flutter && flutter analyze lib/features/search/
```
  </verify>
  <done>
- flutter analyze 无错误
- 提交完成
- VERIFICATION.md 更新
  </done>
</task>

</tasks>

<verification>
## 功能验证

1. **组合模式切换:**
   - 切换到组合模式
   - 确认看到 MultiKeywordInput 组件
   - 确认看到 AND/OR/NOT 按钮

2. **关键词输入:**
   - 输入 "error"
   - 添加 "warning"
   - 确认 Chip 正确显示

3. **条件预览:**
   - 确认搜索按钮上方显示 "error AND warning"
   - 切换为 OR，确认显示 "error OR warning"

4. **搜索执行:**
   - 点击搜索
   - 确认调用 searchStructured API
   - 结果正确显示
</verification>

<success_criteria>
- [ ] imports 添加正确
- [ ] combined 模式显示 MultiKeywordInput
- [ ] SearchConditionPreview 显示条件预览
- [ ] searchQueryProvider 状态管理正常
- [ ] flutter analyze 无错误
</success_criteria>

<output>
After completion, update `.planning/phases/09-advanced-search-ui/09-VERIFICATION.md` status to PASSED.
</output>

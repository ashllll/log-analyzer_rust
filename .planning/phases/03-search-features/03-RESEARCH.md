# Phase 3: 搜索功能与结果展示 - Research

**Researched:** 2026-03-02
**Domain:** Flutter 搜索 UI 与 Rust 后端集成
**Confidence:** HIGH

## Summary

Phase 3 实现了日志搜索功能的核心展示层。基于代码分析，Flutter 前端已有完整的搜索页面框架（SliverFixedExtentList 虚拟滚动 + 关键词高亮），Rust 后端提供了 `search_logs` 和 `async_search_logs` 命令及完整的事件流（搜索结果/进度/摘要/完成）。

**主要差距:**
1. 缺少全屏详情面板（UI-02）
2. 缺少任务进度显示组件（UI-03）
3. 日期范围筛选需从文本输入改为日期选择器
4. 键盘快捷键（Ctrl+F）聚焦搜索框未实现

**主要建议:** 利用现有事件流服务实现进度显示，使用 Flutter 内置 DatePickerDialog 增强日期选择，详情面板采用 Dialog + InfiniteContextViewer 模式。

---

<user_constraints>

## User Constraints (from CONTEXT.md)

### Locked Decisions

#### 搜索结果布局
- 列表布局
- 每行显示: 文件名 + 原始时间戳 + 日志级别 + 完整匹配日志行
- 降序排列 (最新优先)
- 无限滚动加载，每次至少 500 条

#### 关键词高亮样式
- 不同关键词使用不同颜色 (随机分配)
- 背景保持暗色
- 精确匹配高亮
- 点击高亮关键词跳转到详情

#### 筛选 UI 设计
- 位置: 搜索框下方
- 日期范围: 日期选择器，范围自动更新为日志文件的总时间范围
- 筛选实时生效
- 显示重置按钮
- 显示当前筛选条件摘要
- 搜索框: 点击搜索 (非实时搜索)

#### 空状态与加载
- 无结果: 友好提示 + 调整搜索词建议
- 搜索中: 进度条 + 预估时间
- 失败: 简单错误提示
- 首次: "输入关键词开始搜索" 提示

#### 日志详情面板
- 全屏详情视图
- 显示完整日志行 + 无限上下文
- Esc 键关闭
- 列表点击切换详情

#### 任务进度显示
- 页面顶部固定显示
- 显示: 进度条 + 已扫描文件数 + 已找到结果数
- 显示取消按钮
- 完成后显示"完成"状态，几秒后消失

#### 搜索框设计
- 页面顶部通栏
- 自适应宽度
- Ctrl+F / Cmd+F 聚焦搜索框
- 不显示清空按钮

### Claude's Discretion
- 具体的颜色配色方案
- 进度条的具体样式 (线性/圆形)
- 筛选条件的默认状态
- 无限滚动的性能优化策略

### Deferred Ideas (OUT OF SCOPE)
- 日志级别筛选 — 用户选择此阶段不实现
- 文件类型筛选 — 用户选择此阶段不实现

</user_constraints>

---

<phase_requirements>

## Phase Requirements

| ID | Description | Research Support |
|----|-------------|-----------------|
| SEARCH-01 | 用户可以输入关键词进行全文搜索 | 现有 SearchPage + search_logs API 已支持 |
| SEARCH-02 | 搜索结果中高亮显示匹配的关键词 | 现有 LogRowWidget + matchedKeywords 已支持 |
| SEARCH-03 | 用户可以按日期范围筛选搜索结果 | 现有 FilterPalette + time_start/end 过滤器，需增强日期选择器 |
| SEARCH-04 | 用户可以按日志级别筛选 | DEFERRED - 不实现 |
| SEARCH-05 | 用户可以按文件类型筛选 | DEFERRED - 不实现 |
| SEARCH-06 | 搜索响应时间 <200ms | 后端 Aho-Corasick + LRU 缓存已支持 |
| UI-01 | 用户可以看到搜索结果列表 | 现有 SliverFixedExtentList 虚拟滚动已支持 |
| UI-02 | 用户可以查看单条日志详情 | 需实现全屏详情面板 + 无限上下文 |
| UI-03 | 用户可以查看任务进度 | 需实现顶部进度显示组件 |

</phase_requirements>

---

## Standard Stack

### Core

| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| Flutter | 3.x | UI 框架 | 项目主框架 |
| flutter_riverpod | 2.x | 状态管理 | 项目选用 |
| SliverFixedExtentList | Flutter 内置 | 虚拟滚动 | 高性能列表渲染，10K+ 条目 60FPS |
| Text.rich | Flutter 内置 | 关键词高亮 | 已有实现，基于 hash 分配颜色 |

### Supporting

| Library | Purpose | When to Use |
|---------|---------|-------------|
| DatePickerDialog | 日期选择器 | 替换 FilterPalette 中的文本输入 |
| showDialog | 详情面板 | 实现全屏详情视图 |
| KeyboardListener | 键盘事件 | 实现 Ctrl+F 快捷键 |
| StreamBuilder | 响应式 UI | 监听搜索进度事件 |

### Alternatives Considered

| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| 第三方日期选择器 | flutter_date_picker | 内置 DatePickerDialog 足够，且无额外依赖 |
| 详情页路由 | Dialog Overlay | Dialog 更轻量，支持快速切换 |
| Provider 状态 | Riverpod | 项目已用 Riverpod，无需更换 |

---

## Architecture Patterns

### Recommended Project Structure

```
lib/features/search/
├── presentation/
│   ├── search_page.dart       # 已有，需增强
│   └── widgets/
│       ├── log_row_widget.dart    # 已有
│       ├── search_stats_panel.dart # 已有
│       ├── filter_palette.dart    # 已有，需增强日期选择器
│       ├── heatmap_minimap.dart   # 已有
│       ├── search_progress_bar.dart  # 新增 - 任务进度显示
│       └── log_detail_panel.dart    # 新增 - 全屏详情面板
```

### Pattern 1: Event-Driven Progress Display

**What:** 监听 EventStreamService 的搜索事件，实时更新进度 UI

**When to use:** 需要显示长时间运行的搜索任务进度

**Example:**
```dart
// 监听搜索进度
ref.listen(eventStreamServiceProvider, (previous, next) {
  next.searchProgress.listen((progress) {
    // 更新进度条
  });
});
```

### Pattern 2: Dialog-based Detail Panel

**What:** 使用 Dialog 展示日志详情，支持无限上下文加载

**When to use:** 用户需要查看单条日志的完整上下文

**Example:**
```dart
void _showDetailPanel(LogEntry entry) {
  showDialog(
    context: context,
    builder: (context) => LogDetailPanel(
      entry: entry,
      onClose: () => Navigator.pop(context),
    ),
  );
}
```

### Pattern 3: Keyboard Shortcut Handler

**What:** 使用 Focus + KeyboardListener 监听 Ctrl+F

**When to use:** 需要快速聚焦搜索框

**Example:**
```dart
KeyboardListener(
  focusNode: _focusNode,
  onKeyEvent: (event) {
    if (event.logicalKey == LogicalKeyboardKey.keyF &&
        (HardwareKeyboard.instance.isControlPressed ||
         HardwareKeyboard.instance.isMetaPressed)) {
      _searchFocusNode.requestFocus();
    }
  },
)
```

---

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| 虚拟滚动 | 手写列表优化 | SliverFixedExtentList | Flutter 原生支持，O(1) 视口锚点，60FPS |
| 关键词高亮 | 手写 TextSpan | Text.rich + 现有实现 | 已有完整实现 |
| 日期选择 | 手写日历 UI | DatePickerDialog | Flutter 内置，足够需求 |
| 状态管理 | 手写状态 | Riverpod | 项目已采用 |

---

## Common Pitfalls

### Pitfall 1: 搜索结果累积导致内存暴涨

**What goes wrong:** 搜索结果持续累积在 List 中，大数据集下内存溢出

**Why it happens:** EventStreamService 持续 emit 结果，UI 持续添加

**How to avoid:** 使用分页加载，每次只保留 500 条在内存中，后端已有 batch_size=500

**Warning signs:** 日志数量 > 10,000 时明显卡顿

### Pitfall 2: 详情面板加载过多上下文

**What goes wrong:** 无限上下文一次性加载全部历史记录

**Why it happens:** 未实现按需加载机制

**How to avoid:** 使用滑动窗口加载，每次加载前后 100 行

### Pitfall 3: 搜索防抖与用户意图冲突

**What goes wrong:** 用户点击搜索按钮后，防抖定时器延迟执行

**Why it happens:** 搜索栏使用防抖处理 onChanged，但 CONTEXT 要求点击搜索

**How to avoid:** 点击搜索按钮立即执行，不走防抖逻辑

---

## Code Examples

### Enhanced Date Range Picker

```dart
// 使用 Flutter 内置 DatePickerDialog 替换文本输入
Future<DateTimeRange?> _selectDateRange() async {
  final initialDateRange = DateTimeRange(
    start: DateTime.now().subtract(const Duration(days: 7)),
    end: DateTime.now(),
  );

  return showDateRangePicker(
    context: context,
    firstDate: DateTime(2020),
    lastDate: DateTime.now(),
    initialDateRange: initialDateRange,
    builder: (context, child) {
      return Theme(
        data: Theme.of(context).copyWith(
          colorScheme: const ColorScheme.dark(
            primary: AppColors.primary,
            onPrimary: Colors.white,
            surface: AppColors.bgCard,
            onSurface: AppColors.textPrimary,
          ),
        ),
        child: child!,
      );
    },
  );
}
```

### Progress Display Component

```dart
class SearchProgressBar extends StatelessWidget {
  final int progress;           // 0-100
  final int scannedFiles;
  final int resultsFound;
  final VoidCallback? onCancel;

  @override
  Widget build(BuildContext context) {
    return Container(
      padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 8),
      decoration: const BoxDecoration(
        color: AppColors.bgCard,
        border: Border(bottom: BorderSide(color: AppColors.border)),
      ),
      child: Row(
        children: [
          // 进度条
          Expanded(
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                LinearProgressIndicator(
                  value: progress / 100,
                  backgroundColor: AppColors.bgMain,
                  valueColor: const AlwaysStoppedAnimation(AppColors.primary),
                ),
                const SizedBox(height: 4),
                Text(
                  '已扫描 $scannedFiles 文件，已找到 $resultsFound 条结果',
                  style: const TextStyle(fontSize: 12, color: AppColors.textMuted),
                ),
              ],
            ),
          ),
          // 取消按钮
          if (onCancel != null)
            IconButton(
              icon: const Icon(Icons.close),
              onPressed: onCancel,
              tooltip: '取消搜索',
            ),
        ],
      ),
    );
  }
}
```

### Detail Panel with Infinite Context

```dart
class LogDetailPanel extends StatefulWidget {
  final LogEntry entry;
  final VoidCallback onClose;

  const LogDetailPanel({
    super.key,
    required this.entry,
    required this.onClose,
  });

  @override
  State<LogDetailPanel> createState() => _LogDetailPanelState();
}

class _LogDetailPanelState extends State<LogDetailPanel> {
  List<String> _contextLines = [];
  bool _loading = true;

  @override
  void initState() {
    super.initState();
    _loadContext();
  }

  Future<void> _loadContext() async {
    // TODO: 从后端加载前后 100 行上下文
    // 使用 async_search_logs 获取文件内容，然后切片
    setState(() => _loading = false);
  }

  @override
  Widget build(BuildContext context) {
    return Dialog(
      child: KeyboardListener(
        focusNode: FocusNode(),
        onKeyEvent: (event) {
          if (event.logicalKey == LogicalKeyboardKey.escape) {
            widget.onClose();
          }
        },
        child: Container(
          width: MediaQuery.of(context).size.width * 0.9,
          height: MediaQuery.of(context).size.height * 0.9,
          padding: const EdgeInsets.all(16),
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              // 标题栏
              Row(
                children: [
                  Text(widget.entry.file, style: const TextStyle(fontSize: 18, fontWeight: FontWeight.bold)),
                  const Spacer(),
                  IconButton(icon: const Icon(Icons.close), onPressed: widget.onClose),
                ],
              ),
              const Divider(),
              // 上下文内容
              Expanded(
                child: _loading
                    ? const Center(child: CircularProgressIndicator())
                    : ListView.builder(
                        itemCount: _contextLines.length,
                        itemBuilder: (context, index) {
                          final isMatch = _contextLines[index].contains(widget.entry.content);
                          return Container(
                            color: isMatch ? AppColors.bgHover : null,
                            padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 2),
                            child: Text(
                              _contextLines[index],
                              style: const TextStyle(fontFamily: 'FiraCode', fontSize: 13),
                            ),
                          );
                        },
                      ),
              ),
            ],
          ),
        ),
      ),
    );
  }
}
```

---

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| ListView.builder | SliverFixedExtentList | Flutter 2.x | O(1) 视口锚点，60FPS 虚拟滚动 |
| 自定义高亮 | Text.rich + hash 颜色 | 已有实现 | 简洁高效 |
| 文本日期输入 | DatePickerDialog | Flutter 内置 | 更友好，更准确 |

**Deprecated/outdated:**
- 实时搜索（onChanged 防抖）: 已改为点击搜索，满足 CONTEXT 要求

---

## Open Questions

1. **详情面板的上下文数据源**
   - What we know: 后端 search_logs 返回 LogEntry.content (单行)，需要获取文件全文
   - What's unclear: 后端是否已有按行号范围获取上下文的 API？
   - Recommendation: 检查 Rust 后端是否有 getFileContentRange 或类似命令

2. **日期范围自动更新**
   - What we know: CONTEXT 要求日期范围自动更新为日志文件的总时间范围
   - What's unclear: 后端是否已实现获取工作区日志时间范围的 API？
   - Recommendation: 需要在 Phase 3 或之前添加 getWorkspaceTimeRange 命令

3. **进度显示数据源**
   - What we know: 搜索进度通过事件流发送，包含 progress, scanned_files, results_count
   - What's unclear: 事件流是否包含预估剩余时间？
   - Recommendation: 在实现进度组件时考虑预留时间显示字段

---

## Validation Architecture

### Test Framework
| Property | Value |
|----------|-------|
| Framework | flutter_test (Dart) |
| Config file | None — 使用标准 flutter test |
| Quick run command | `flutter test test/features/search/` |
| Full suite command | `flutter test` |

### Phase Requirements → Test Map
| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|---------------|
| SEARCH-01 | 搜索功能 | unit | `flutter test test/search_test.dart` | TODO |
| SEARCH-02 | 关键词高亮 | unit | `flutter test test/highlight_test.dart` | TODO |
| SEARCH-03 | 日期筛选 | widget | `flutter test test/filter_palette_test.dart` | TODO |
| UI-01 | 结果列表显示 | widget | `flutter test test/log_list_test.dart` | TODO |
| UI-02 | 详情面板 | widget | `flutter test test/detail_panel_test.dart` | TODO |
| UI-03 | 进度显示 | widget | `flutter test test/progress_bar_test.dart` | TODO |

### Sampling Rate
- **Per task commit:** `flutter test test/features/search/`
- **Per wave merge:** `flutter test`
- **Phase gate:** Full suite green before `/gsd:verify-work`

### Wave 0 Gaps
- [ ] `test/features/search/search_test.dart` — 覆盖 SEARCH-01, SEARCH-06
- [ ] `test/features/search/highlight_test.dart` — 覆盖 SEARCH-02
- [ ] `test/features/search/filter_palette_test.dart` — 覆盖 SEARCH-03
- [ ] `test/features/search/log_detail_panel_test.dart` — 覆盖 UI-02
- [ ] `test/features/search/progress_bar_test.dart` — 覆盖 UI-03

---

## Sources

### Primary (HIGH confidence)
- Flutter 官方文档: SliverFixedExtentList, DatePickerDialog, Dialog
- 项目现有代码: search_page.dart, log_row_widget.dart, filter_palette.dart
- Rust 后端: search.rs, async_search.rs 命令实现
- EventStreamService: 搜索事件流处理

### Secondary (MEDIUM confidence)
- 现有热力图缩略图实现: heatmap_minimap.dart
- 现有统计面板实现: search_stats_panel.dart

---

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH - Flutter/Riverpod 已在使用，组件基于现有代码
- Architecture: HIGH - 遵循现有 Flutter + Riverpod 模式
- Pitfalls: MEDIUM - 基于现有实现经验，可能需实际验证

**Research date:** 2026-03-02
**Valid until:** 2026-04-02 (30 days for stable Flutter ecosystem)

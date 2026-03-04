# Feature Research - v1.1 高级搜索与虚拟文件系统

**Domain:** Flutter Desktop Log Analyzer - Advanced Search & Virtual File System
**Researched:** 2026-03-04
**Confidence:** HIGH

## Executive Summary

This research focuses on v1.1 milestone features: regex search, multi-keyword combined search (AND/OR/NOT), search history, and virtual file system. The Rust backend already has most capabilities implemented (regex via `regex-automata`, Aho-Corasick for multi-pattern, search history commands). The main work is building Flutter UI for these features.

## Key Findings

**Backend Capabilities (Already Implemented):**
- Regex search: `regex-automata` DFA engine (stream matching)
- Multi-keyword: Aho-Corasick algorithm (O(n+m) complexity)
- AND/OR/NOT operators: PatternMatcher service
- Search history: `add_search_history()`, `get_search_history()`, `delete_search_history()` commands
- Virtual file tree: `get_virtual_file_tree()` command exists

**Frontend Status:**
- Basic search UI exists (search_page.dart)
- Filter palette exists
- Search history related code exists in settings
- Virtual file tree NOT implemented yet

---

## Table Stakes (必备功能)

功能缺失会让产品感觉不完整，用户认为理所当然。

| Feature | Why Expected | Complexity | Implementation Notes |
|---------|--------------|------------|---------------------|
| 正则表达式搜索 | 日志分析需要复杂模式匹配 (IP、日期、堆栈) | MEDIUM | 后端 DFA 引擎已实现，前端需添加 regex 模式选择器 UI |
| 多关键词组合 (AND/OR/NOT) | 精确缩小搜索范围是日志分析核心需求 | MEDIUM | 后端 PatternMatcher 已支持，前端需构建查询构建器 UI |
| 搜索历史记录 | 重复搜索相同关键词是常见工作流 | LOW | 后端命令已实现，前端需集成显示组件 |
| 搜索历史快速访问 | 点击历史记录快速填充搜索框 | LOW | 依赖搜索历史，需添加下拉建议 UI |
| 虚拟文件树导航 | 浏览归档内文件结构 | MEDIUM | 后端 `get_virtual_file_tree` 存在，需实现 Flutter TreeView |
| 目录层级浏览 | 在虚拟文件系统中导航 | LOW | 依赖虚拟文件树，需展开/折叠交互 |

---

## Differentiators (差异化功能)

让产品脱颖而出的功能，非必需但有高价值。

| Feature | Value Proposition | Complexity | Implementation Notes |
|---------|-------------------|------------|---------------------|
| 智能搜索建议 | 基于历史和上下文自动补全 | MEDIUM | 可利用搜索历史数据，构建简单推荐算法 |
| 搜索语法高亮 | 输入时显示搜索语法状态 (有效/无效) | LOW | 正则表达式实时验证，UI 反馈 |
| 搜索结果实时预览 | 输入时渐进显示结果 (需性能优化) | HIGH | 防抖 + 流式结果，需后端支持 |
| 文件树搜索过滤 | 在大型虚拟文件树中快速定位 | LOW | 在 TreeView 上添加搜索过滤 |
| 最近文件快速访问 | 显示最近打开/搜索的文件 | LOW | 依赖搜索历史，按时间排序 |

---

## Anti-Features (避免构建的功能)

看起来很好但会产生问题的功能。

| Feature | Why Avoid | Alternative |
|---------|-----------|-------------|
| 实时正则验证 (大文件) | 复杂正则可能导致 UI 卡顿 | 使用 debounce + 后端异步验证 |
| 无限搜索历史 | 存储空间浪费，数据噪音 | 限制历史数量 (默认 100 条) |
| 云端搜索历史同步 | 本地应用不需要，隐私问题 | 保持本地存储 |
| 复杂文件树动画 | Flutter 桌面端性能开销 | 简单展开/折叠即可 |

---

## Feature Dependencies

```
[正则表达式搜索]
    └──requires──> [搜索模式选择 UI]
    └──requires──> [正则语法验证]

[多关键词组合搜索]
    └──requires──> [关键词输入组件]
    └──requires──> [AND/OR/NOT 选择器]
    └──requires──> [搜索条件预览]

[搜索历史记录]
    └──requires──> [后端 search_history 命令]
    └──requires──> [历史列表 UI]
    └──enhances──> [智能搜索建议]

[虚拟文件树]
    └──requires──> [后端 get_virtual_file_tree]
    └──requires──> [TreeView 组件]
    └──requires──> [文件/目录图标]
    └──requires──> [展开/折叠状态管理]

[目录层级浏览]
    └──requires──> [虚拟文件树]
    └──requires──> [点击目录加载子项]
```

---

## User Behavior Analysis

### 搜索行为模式

1. **快速搜索 (60%)**
   - 输入单一关键词
   - 期望 < 200ms 结果
   - 行为: 输入 → 等待 → 滚动结果 → 定位问题

2. **精确搜索 (25%)**
   - 使用 AND/OR/NOT 组合
   - 使用正则表达式
   - 行为: 切换模式 → 构建查询 → 执行 → 细化

3. **重复搜索 (15%)**
   - 搜索相同或相似关键词
   - 行为: 查看历史 → 点击历史 → 轻微调整 → 执行

### 虚拟文件系统行为

1. **归档浏览 (70%)**
   - 展开目录 → 找到目标文件 → 点击查看内容
   - 行为: 层级导航 → 叶子节点 → 预览

2. **批量操作 (30%)**
   - 选择多个文件
   - 导入整个目录
   - 行为: 多选 → 操作菜单 → 执行

### 用户期望

| 场景 | 用户期望 |
|------|----------|
| 输入正则表达式 | 即时语法反馈，有效显示绿色，无效显示红色 |
| 组合多个关键词 | 清晰显示组合关系 (A AND B OR C) |
| 查看搜索历史 | 按时间排序，支持删除单条 |
| 浏览虚拟文件树 | 展开/折叠流畅，图标区分文件/目录 |
| 大目录导航 | 支持键盘导航 (上下箭头 + 回车) |

---

## MVP Recommendation

### Launch With (v1.1)

最小可行功能集 - 验证高级搜索和虚拟文件系统价值。

**优先级 1 (必须):**
- [ ] 正则表达式搜索模式切换 (简单 toggle)
- [ ] AND/OR/NOT 关键词组合 UI
- [ ] 搜索历史下拉列表
- [ ] 虚拟文件树基础展示 (TreeView)

**优先级 2 (完善):**
- [ ] 搜索语法实时验证反馈
- [ ] 历史记录删除功能
- [ ] 目录展开/折叠交互
- [ ] 虚拟文件树文件预览

### Add After Validation (v1.x)

功能验证后添加。

- [ ] 智能搜索建议 (基于历史)
- [ ] 搜索结果实时预览 (debounced)
- [ ] 虚拟文件树搜索过滤
- [ ] 最近文件快速访问

---

## Complexity Assessment

| Feature | Frontend Complexity | Backend Complexity | Integration Effort |
|---------|--------------------|--------------------|-------------------|
| Regex Search Toggle | LOW | LOW (已有) | LOW |
| Multi-keyword UI | MEDIUM | LOW (已有) | MEDIUM |
| Search History UI | LOW | LOW (已有) | LOW |
| Virtual File Tree | MEDIUM | MEDIUM | MEDIUM |
| Tree Navigation | LOW | LOW | LOW |

---

## Implementation Recommendations

### 1. 正则表达式搜索 UI

```dart
// 推荐实现: 搜索栏添加模式切换
Row(
  children: [
    Expanded(child: SearchTextField(...)),
    ToggleButtons(
      isSelected: [isKeyword, isRegex],
      onPressed: (index) => setSearchMode(index),
      children: [
        Tooltip(message: '关键词搜索', child: Icon(Icons.search)),
        Tooltip(message: '正则表达式', child: Icon(Icons.code)),
      ],
    ),
  ],
)
```

### 2. 多关键词组合 UI

```dart
// 推荐实现: Chip + Operator 选择器
Wrap(
  spacing: 8,
  children: [
    Chip(label: Text('error')),
    Chip(label: Text('AND')),
    Chip(label: Text('database')),
    Chip(label: Text('OR')),
    Chip(label: Text('timeout')),
  ],
)
```

### 3. 搜索历史 UI

```dart
// 推荐实现: 搜索栏下拉建议
Autocomplete<SearchHistoryEntry>(
  optionsBuilder: (textEditingValue) {
    return history.where((e) => e.query.contains(textEditingValue.text));
  },
  onSelected: (entry) => searchController.text = entry.query,
)
```

### 4. 虚拟文件树 UI

```dart
// 推荐实现: 递归 TreeView
class VirtualFileTree extends StatelessWidget {
  final List<VirtualFileNode> nodes;

  @override
  Widget build(BuildContext context) {
    return ListView.builder(
      itemCount: nodes.length,
      itemBuilder: (context, index) => _buildNode(nodes[index]),
    );
  }

  Widget _buildNode(VirtualFileNode node) {
    if (node.isDirectory) {
      return ExpansionTile(
        leading: Icon(Icons.folder),
        title: Text(node.name),
        children: node.children.map(_buildNode).toList(),
      );
    }
    return ListTile(
      leading: Icon(Icons.description),
      title: Text(node.name),
    );
  }
}
```

---

## Backend API Summary

| Command | Status | Flutter Integration |
|---------|--------|---------------------|
| `search_logs(query, mode)` | 已有 | 需添加 mode 参数 |
| `add_search_history(...)` | 已有 | 需调用 |
| `get_search_history(...)` | 已有 | 需集成 UI |
| `delete_search_history(id)` | 已有 | 需添加删除按钮 |
| `get_virtual_file_tree(workspace_id)` | 已有 | 需实现 TreeView |

---

## Sources

- PROJECT.md - 项目需求和 v1.1 里程碑定义
- CLAUDE.md - Rust 后端能力说明
- search_history.rs - 后端搜索历史命令实现
- search_page.dart - 现有 Flutter 搜索页面
- services/pattern_matcher.rs - Aho-Corasick 多模式匹配
- search_engine/dfa_engine.rs - DFA 正则引擎

---

*Research for: Flutter Desktop Log Analyzer v1.1*
*Researched: 2026-03-04*

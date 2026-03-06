# Phase 10: 虚拟文件系统 UI - Research

**Researched:** 2026-03-07
**Domain:** Flutter 树形视图组件 + 文件预览 + 状态管理
**Confidence:** HIGH

## Summary

Phase 10 需要实现虚拟文件系统 UI，包括：
1. 树形视图组件展示虚拟文件树
2. 目录展开/折叠功能（懒加载子节点）
3. 文件预览功能
4. 与现有 SearchPage 通过标签页集成

研究发现：
- **Flutter SDK >=3.8.0** 可使用官方 **TreeSliver** widget（Flutter 3.24.0+ 引入）
- 现有 **VirtualFileTreeProvider** 已实现懒加载 API（loadChildren、readFileByHash）
- 项目使用 **Riverpod 3.0** 进行状态管理
- **flutter_treeview** 库是成熟替代方案，支持更丰富的自定义功能

**Primary recommendation:** 使用 Flutter 官方 **TreeSliver** + 自定义 TreeSliverNode 组件实现树形视图，或使用 **flutter_treeview** 库获得更丰富的功能。

---

<user_constraints>

## User Constraints (from CONTEXT.md)

### Locked Decisions

- **位置**: 左侧边栏 + 主预览区，类似 VS Code 的经典 IDE 布局
- **行高密度**: 紧凑模式，每行 24-28px，适合大型工作区
- **侧边栏宽度**: 可拖动调整，最小 200px，最大 500px
- **预览面板**: 标签页切换模式，保留原有日志列表，用户通过标签切换
- **点击行为**: 单击选中并预览，右键显示上下文菜单
- **键盘导航**: 完整支持（上下箭头导航，左右箭头折叠/展开，回车打开预览）
- **多选支持**: Ctrl+点击 和 Shift+点击 多选文件/目录
- **节点信息**: 仅显示文件名（鼠标悬停显示完整路径 tooltip）
- **图标风格**: 文件类型图标，根据扩展名区分（.log, .txt, .json, .zip 等）
- **预览内容**: 显示文件内容（纯文本），支持滚动查看
- **语法高亮**: 纯文本显示，无语法高亮（适合日志文件）
- **空状态**: 友好空状态，显示图标 + 文案 "工作区为空，导入文件开始分析"
- **加载状态**: 骨架屏（Skeleton）动画效果
- **错误状态**: 使用现有的 ErrorView 组件，显示错误信息和重试按钮

### Claude's Discretion

- 具体的骨架屏样式和动画
- 文件类型图标的具体设计
- 错误信息的具体文案
- 加载指示器的样式

### Deferred Ideas (OUT OF SCOPE)

None - discussion stayed within phase scope

</user_constraints>

---

<phase_requirements>

## Phase Requirements

| ID | Description | Research Support |
|----|-------------|-----------------|
| VFS-01 | 用户可以查看工作区的虚拟文件树结构 | TreeSliver + VirtualFileTreeProvider 集成 |
| VFS-02 | 目录节点可以展开/折叠 | TreeSliverController + 懒加载 API |
| VFS-03 | 用户可以点击文件预览内容 | readFileByHash API + PreviewPanel |
| VFS-04 | 文件树显示文件/目录图标区分 | 自定义 TreeSliverNodeBuilder + 图标映射 |

</phase_requirements>

---

## Standard Stack

### Core

| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| **TreeSliver** (官方) | Flutter 3.24.0+ | 树形视图组件 | Flutter 官方组件，虚拟滚动性能好，与 CustomScrollView 集成 |
| **flutter_treeview** | latest | 树形视图备选库 | 功能更丰富，支持更多自定义选项 |
| **lucide_icons_flutter** | ^1.0.0 | 文件类型图标 | 项目已使用，与 React 版本一致 |

### Supporting

| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| **Riverpod 3.0** | ^3.0.0 | 状态管理 | 项目已配置，用于 VirtualFileTreeProvider |
| **shimmer** | ^3.0.0 | 骨架屏动画 | 加载状态显示（Flutter 官方推荐） |

### Alternatives Considered

| Instead of | Could Use | Tradeoff |
|-----------|-----------|----------|
| TreeSliver (官方) | flutter_treeview | TreeSliver 更轻量，flutter_treeview 功能更丰富（如拖拽、动画） |
| shimmer | skeletonizer | shimmer 更成熟，skeletonizer 更容易使用 |

**Installation:**
```bash
cd log-analyzer_flutter
flutter pub add shimmer
# TreeSliver 是 Flutter SDK 内置，无需安装
# flutter_treeview 作为备选：
flutter pub add flutter_treeview
```

---

## Architecture Patterns

### Recommended Project Structure

```
lib/features/
└── virtual_file_tree/
    ├── presentation/
    │   ├── virtual_file_tree_page.dart    # 页面入口（标签页）
    │   ├── widgets/
    │   │   ├── file_tree_sidebar.dart     # 左侧边栏
    │   │   ├── file_tree_view.dart         # 树形视图组件
    │   │   ├── file_tree_node.dart         # 单个节点组件
    │   │   ├── file_preview_panel.dart     # 预览面板
    │   │   ├── empty_state.dart            # 空状态
    │   │   └── loading_skeleton.dart       # 加载骨架屏
    │   └── controllers/
    │       └── file_tree_controller.dart   # TreeSliverController 封装
    └── providers/
        └── file_tree_ui_provider.dart       # UI 状态管理
```

### Pattern 1: TreeSliver 集成 VirtualFileTreeProvider

**What:** 使用 TreeSliver 展示虚拟文件树，与 VirtualFileTreeProvider 集成实现懒加载

**When to use:** 需要高性能虚拟滚动，项目 Flutter 版本 >= 3.24.0

**Example:**
```dart
// 使用 TreeSliver 实现文件树
class FileTreeView extends StatefulWidget {
  final List<VirtualTreeNode> nodes;
  final void Function(VirtualTreeNode) onNodeTap;
  final Future<void> Function(String parentPath) onLoadChildren;

  const FileTreeView({
    super.key,
    required this.nodes,
    required this.onNodeTap,
    required this.onLoadChildren,
  });
}

class _FileTreeViewState extends State<FileTreeView> {
  late TreeSliverController<String> _controller;

  @override
  void initState() {
    super.initState();
    _controller = TreeSliverController<String>();
  }

  @override
  void dispose() {
    _controller.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    return CustomScrollView(
      slivers: [
        TreeSliver<String>(
          tree: _buildTreeData(widget.nodes),
          builder: (context, entry) {
            final node = entry.value;
            return _FileTreeNodeWidget(
              node: node,
              isExpanded: entry.isExpanded,
              onTap: () => widget.onNodeTap(node),
              onExpand: () => _handleExpand(node),
            );
          },
        ),
      ],
    );
  }

  TreeNode<String> _buildTreeData(List<VirtualTreeNode> nodes) {
    // 将 VirtualTreeNode 转换为 TreeSliver 需要的格式
    return TreeNode<String>(
      data: 'root',
      children: nodes.map((n) => _convertToTreeNode(n)).toList(),
    );
  }

  Future<void> _handleExpand(VirtualTreeNode node) async {
    if (node.needsLazyLoad) {
      await widget.onLoadChildren(node.nodePath);
    }
    _controller.toggleExpansion(node.nodePath);
  }
}
```

### Pattern 2: 侧边栏 + 预览面板布局

**What:** 使用 Row 布局，左侧为可调宽度的文件树侧边栏，右侧为预览面板

**When to use:** 需要类似 VS Code 的经典 IDE 布局

**Example:**
```dart
class FileTreeSidebar extends StatefulWidget {
  const FileTreeSidebar({super.key});

  @override
  State<FileTreeSidebar> createState() => _FileTreeSidebarState();
}

class _FileTreeSidebarState extends State<FileTreeSidebar> {
  double _sidebarWidth = 250;
  static const double _minWidth = 200;
  static const double _maxWidth = 500;

  @override
  Widget build(BuildContext context) {
    return Row(
      children: [
        // 文件树
        SizedBox(
          width: _sidebarWidth,
          child: const FileTreeView(...),
        ),
        // 拖动调整手柄
        GestureDetector(
          onHorizontalDragUpdate: (details) {
            setState(() {
              _sidebarWidth = (_sidebarWidth + details.delta.dx)
                  .clamp(_minWidth, _maxWidth);
            });
          },
          child: MouseRegion(
            cursor: SystemMouseCursors.resizeColumn,
            child: Container(
              width: 4,
              color: Colors.transparent,
            ),
          ),
        ),
        // 预览面板
        const Expanded(child: FilePreviewPanel()),
      ],
    );
  }
}
```

### Pattern 3: 文件预览与内容加载

**What:** 点击文件后异步加载内容，显示加载状态或内容

**When to use:** 文件内容需要从后端获取时

**Example:**
```dart
class FilePreviewPanel extends ConsumerStatefulWidget {
  final VirtualTreeNode? selectedFile;

  const FilePreviewPanel({super.key, this.selectedFile});

  @override
  ConsumerState<FilePreviewPanel> createState() => _FilePreviewPanelState();
}

class _FilePreviewPanelState extends ConsumerState<FilePreviewPanel> {
  AsyncValue<FileContentResponse?> _contentAsync = const AsyncValue.data(null);

  @override
  void didUpdateWidget(FilePreviewPanel oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (widget.selectedFile != oldWidget.selectedFile &&
        widget.selectedFile != null) {
      _loadContent();
    }
  }

  Future<void> _loadContent() async {
    if (widget.selectedFile == null) return;

    setState(() => _contentAsync = const AsyncValue.loading());

    try {
      final content = await ref
          .read(virtualFileTreeProvider(widget.selectedFile!.workspaceId).notifier)
          .readFileByHash(widget.selectedFile!.nodeHash);

      setState(() => _contentAsync = AsyncValue.data(content));
    } catch (e, st) {
      setState(() => _contentAsync = AsyncValue.error(e, st));
    }
  }

  @override
  Widget build(BuildContext context) {
    return _contentAsync.when(
      loading: () => const Center(child: CircularProgressIndicator()),
      error: (e, _) => ErrorView(
        exception: AppException(code: ErrorCodes.unknown, message: e.toString()),
        onRetry: _loadContent,
      ),
      data: (content) => content == null
          ? const Center(child: Text('无法加载文件内容'))
          : _buildContentView(content),
    );
  }

  Widget _buildContentView(FileContentResponse content) {
    return SingleChildScrollView(
      padding: const EdgeInsets.all(16),
      child: SelectableText(
        content.content,
        style: const TextStyle(
          fontFamily: 'monospace',
          fontSize: 13,
        ),
      ),
    );
  }
}
```

### Pattern 4: 骨架屏加载状态

**What:** 使用 shimmer 实现加载状态的骨架屏动画

**When to use:** 数据加载中显示占位符

**Example:**
```dart
import 'package:shimmer/shimmer.dart';

class FileTreeLoadingSkeleton extends StatelessWidget {
  const FileTreeLoadingSkeleton({super.key});

  @override
  Widget build(BuildContext context) {
    return Shimmer.fromColors(
      baseColor: Colors.grey[800]!,
      highlightColor: Colors.grey[700]!,
      child: ListView.builder(
        itemCount: 10,
        itemBuilder: (context, index) => Padding(
          padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 8),
          child: Row(
            children: [
              Container(
                width: 16,
                height: 16,
                decoration: BoxDecoration(
                  color: Colors.white,
                  borderRadius: BorderRadius.circular(4),
                ),
              ),
              const SizedBox(width: 8),
              Container(
                width: 120,
                height: 14,
                decoration: BoxDecoration(
                  color: Colors.white,
                  borderRadius: BorderRadius.circular(4),
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

### Anti-Patterns to Avoid

- **在 TreeView 中直接加载所有子节点**: 应使用懒加载，展开目录时才调用 `loadChildren`
- **文件内容直接在主线程加载**: 应使用 FutureBuilder 或 AsyncValue 处理异步加载
- **不使用虚拟滚动**: 大型文件树（1000+ 节点）必须使用 TreeSliver 或等效虚拟滚动方案

---

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| 树形视图 | 手写递归 ListView | TreeSliver / flutter_treeview | 虚拟滚动性能好，展开/折叠状态管理复杂 |
| 骨架屏动画 | 自定义 AnimationController | shimmer 包 | 成熟稳定，社区认可 |
| 文件类型图标 | 自定义图标映射 | lucide_icons_flutter + 自定义扩展 | 项目已使用，风格一致 |

**Key insight:** 树形视图的状态管理（展开/折叠、选中状态、多选）非常复杂，手写容易出现性能问题和状态不一致。使用官方 TreeSliver 或成熟的 flutter_treeview 库可以避免这些问题。

---

## Common Pitfalls

### Pitfall 1: 懒加载状态不同步
**What goes wrong:** 目录展开后，子节点加载完成但 UI 未更新

**Why it happens:** `loadChildren` 是异步的，状态更新时机不对

**How to avoid:** 使用 Riverpod AsyncValue 管理加载状态，确保在 Provider 状态更新后才刷新 UI

**Warning signs:** 展开目录后 UI 无变化或闪烁

### Pitfall 2: 大型文件树性能问题
**What goes wrong:** 1000+ 文件节点时滚动卡顿

**Why it happens:** 未使用虚拟滚动，每次渲染所有节点

**How to avoid:** 使用 TreeSliver 或 ListView.builder 配合懒加载

**Warning signs:** 日志中出现掉帧，滚动不流畅

### Pitfall 3: 侧边栏宽度调整与布局冲突
**What goes wrong:** 拖动调整侧边栏宽度时，右侧预览面板内容溢出或布局错乱

**Why it happens:** 未正确使用 Expanded 或 Flexible 包装预览面板

**How to avoid:** 正确使用 Row + Expanded 布局组合

### Pitfall 4: 文件内容加载失败未处理
**What goes wrong:** 点击文件后显示空白，未提示用户错误

**Why it happens:** 未使用 AsyncValue.error 处理异常情况

**How to avoid:** 使用 `_contentAsync.when()` 完整处理 loading/error/data 三种状态

---

## Code Examples

Verified patterns from official sources:

### 文件类型图标映射
```dart
import 'package:lucide_icons_flutter/lucide_icons.dart';

// 文件扩展名到图标的映射
IconData getFileIcon(String fileName) {
  final ext = fileName.split('.').last.toLowerCase();
  switch (ext) {
    case 'log':
      return LucideIcons.fileText;
    case 'txt':
      return LucideIcons.fileText;
    case 'json':
      return LucideIcons.fileJson;
    case 'xml':
      return LucideIcons.fileCode;
    case 'zip':
    case 'tar':
    case 'gz':
    case 'rar':
    case '7z':
      return LucideIcons.archive;
    case 'pdf':
      return LucideIcons.fileText;
    case 'jpg':
    case 'jpeg':
    case 'png':
    case 'gif':
    case 'bmp':
      return LucideIcons.image;
    default:
      return LucideIcons.file;
  }
}

// 目录图标
const IconData directoryIcon = LucideIcons.folder;

// 展开目录图标
const IconData directoryOpenIcon = LucideIcons.folderOpen;
```

### 键盘导航支持
```dart
// 使用 RawKeyboardListener 或 KeyboardListener
KeyboardListener(
  focusNode: FocusNode(),
  autofocus: true,
  onKeyEvent: (event) {
    if (event is KeyDownEvent) {
      switch (event.logicalKey) {
        case LogicalKeyboardKey.arrowDown:
          _selectNextNode();
          break;
        case LogicalKeyboardKey.arrowUp:
          _selectPreviousNode();
          break;
        case LogicalKeyboardKey.arrowRight:
          if (_selectedNode.isDirectory && !_selectedNode.isExpanded) {
            _expandNode();
          }
          break;
        case LogicalKeyboardKey.arrowLeft:
          if (_selectedNode.isExpanded) {
            _collapseNode();
          }
          break;
        case LogicalKeyboardKey.enter:
          _openPreview();
          break;
      }
    }
  },
  child: FileTreeView(...),
)
```

### 多选支持 (Ctrl+点击 / Shift+点击)
```dart
class FileTreeSelectionController {
  final Set<String> _selectedPaths = {};
  String? _anchorPath; // Shift 点击的锚点

  void handleTap(String path, bool isCtrlPressed, bool isShiftPressed) {
    if (isCtrlPressed) {
      // Ctrl+点击：切换选中状态
      if (_selectedPaths.contains(path)) {
        _selectedPaths.remove(path);
      } else {
        _selectedPaths.add(path);
        _anchorPath = path;
      }
    } else if (isShiftPressed && _anchorPath != null) {
      // Shift+点击：范围选中
      _selectRange(_anchorPath!, path);
    } else {
      // 普通点击：单选
      _selectedPaths.clear();
      _selectedPaths.add(path);
      _anchorPath = path;
    }
  }

  bool isSelected(String path) => _selectedPaths.contains(path);
}
```

---

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| 自定义递归 ListView | TreeSliver (官方) | Flutter 3.24.0 | 虚拟滚动性能更好，与 CustomScrollView 集成 |
| 手写展开/折叠状态 | TreeSliverController | Flutter 3.24.0 | 简化状态管理，支持编程控制 |
| AnimationController 动画 | shimmer 包 | 广泛使用 | 骨架屏更稳定，样式更丰富 |

**Deprecated/outdated:**
- **baumths/flutter_tree_view**: 已废弃，推荐使用官方 TreeSliver
- **手动虚拟滚动**: 不推荐，使用官方 TreeSliver 代替

---

## Open Questions

1. **是否需要支持目录节点展开/折叠动画？**
   - What we know: TreeSliver 支持动画，flutter_treeview 提供更丰富的动画选项
   - What's unclear: 用户体验偏好
   - Recommendation: 先实现基本展开/折叠，后续根据需求添加动画

2. **文件树是否需要搜索/过滤功能？**
   - What we know: CONTEXT.md 未明确要求
   - What's unclear: VFS-05 (文件树搜索过滤) 属于 v2 需求
   - Recommendation: Phase 10 不实现，保持简单

3. **预览大文件时是否需要分页？**
   - What we know: readFileByHash 返回完整内容
   - What's unclear: 文件大小限制
   - Recommendation: 先加载完整内容，后续根据实际需求优化

---

## Sources

### Primary (HIGH confidence)
- [Flutter TreeSliver API](https://api.flutter.dev/flutter/widgets/TreeSliver-class.html) - 官方 TreeSliver 文档
- [Flutter TreeSliverController API](https://api.flutter.dev/flutter/widgets/TreeSliverController-class.html) - 控制器文档
- [pub.dev shimmer](https://pub.dev/packages/shimmer) - 骨架屏包
- [lucide_icons_flutter](https://pub.dev/packages/lucide_icons_flutter) - 图标包

### Secondary (MEDIUM confidence)
- [flutter_treeview package](https://pub.dev/packages/flutter_treeview) - 树形视图替代库
- [Flutter TreeSliver 实现示例](https://github.com/flutter/samples/tree/main/experimental/treesliver) - 官方示例

### Tertiary (LOW confidence)
- [WebSearch: Flutter expandable list patterns](https://www.google.com/search?q=Flutter+expandable+list+patterns) - 社区实践

---

## Validation Architecture

> Skip this section entirely if workflow.nyquist_validation is false in .planning/config.json

### Test Framework
| Property | Value |
|----------|-------|
| Framework | flutter_test (内置) |
| Config file | None — see default Flutter test setup |
| Quick run command | `flutter test test/virtual_file_tree_test.dart -x` |
| Full suite command | `flutter test` |

### Phase Requirements -> Test Map
| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| VFS-01 | 用户可以查看工作区的虚拟文件树结构 | Widget | `flutter test test/vfs_test.dart::test_file_tree_renders -x` | TBD |
| VFS-02 | 目录节点可以展开/折叠 | Widget | `flutter test test/vfs_test.dart::test_expand_collapse -x` | TBD |
| VFS-03 | 用户可以点击文件预览内容 | Widget | `flutter test test/vfs_test.dart::test_file_preview -x` | TBD |
| VFS-04 | 文件树显示文件/目录图标区分 | Widget | `flutter test test/vfs_test.dart::test_file_icons -x` | TBD |

### Sampling Rate
- **Per task commit:** `flutter test test/virtual_file_tree_test.dart -x`
- **Per wave merge:** `flutter test`
- **Phase gate:** Full suite green before `/gsd:verify-work`

### Wave 0 Gaps
- [ ] `test/virtual_file_tree_test.dart` — 覆盖 VFS-01~04
- [ ] `test/widgets/file_tree_node_test.dart` — 节点组件测试
- [ ] `test/widgets/file_preview_panel_test.dart` — 预览面板测试

---

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH - 使用 Flutter 官方 TreeSliver，库成熟稳定
- Architecture: HIGH - 基于现有 VirtualFileTreeProvider 集成，模式清晰
- Pitfalls: HIGH - 常见问题已识别，对应方案明确

**Research date:** 2026-03-07
**Valid until:** 2026-04-07 (30 days for stable technology)

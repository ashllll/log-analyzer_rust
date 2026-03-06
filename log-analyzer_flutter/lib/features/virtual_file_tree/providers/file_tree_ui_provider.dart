import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:shared_preferences/shared_preferences.dart';

/// 文件树 UI 状态
///
/// 管理展开/折叠状态、选中节点、多选状态、侧边栏宽度
class FileTreeUIState {
  /// 展开的节点路径集合
  final Set<String> expandedPaths;

  /// 当前选中的节点路径（单选）
  final String? selectedPath;

  /// 多选的节点路径集合
  final Set<String> selectedPaths;

  /// Shift 点击的锚点路径
  final String? anchorPath;

  /// 侧边栏宽度
  final double sidebarWidth;

  /// 侧边栏是否折叠
  final bool isSidebarCollapsed;

  /// 构造函数
  const FileTreeUIState({
    this.expandedPaths = const {},
    this.selectedPath,
    this.selectedPaths = const {},
    this.anchorPath,
    this.sidebarWidth = 280.0,
    this.isSidebarCollapsed = false,
  });

  /// 是否有多选
  bool get hasMultipleSelection => selectedPaths.isNotEmpty;

  /// 获取所有选中的路径（包括单选和多选）
  Set<String> get allSelectedPaths {
    final paths = <String>{...selectedPaths};
    if (selectedPath != null) {
      paths.add(selectedPath!);
    }
    return paths;
  }

  /// 复制
  FileTreeUIState copyWith({
    Set<String>? expandedPaths,
    String? selectedPath,
    Set<String>? selectedPaths,
    String? anchorPath,
    double? sidebarWidth,
    bool? isSidebarCollapsed,
    bool clearSelectedPath = false,
    bool clearAnchorPath = false,
  }) {
    return FileTreeUIState(
      expandedPaths: expandedPaths ?? this.expandedPaths,
      selectedPath: clearSelectedPath ? null : (selectedPath ?? this.selectedPath),
      selectedPaths: selectedPaths ?? this.selectedPaths,
      anchorPath: clearAnchorPath ? null : (anchorPath ?? this.anchorPath),
      sidebarWidth: sidebarWidth ?? this.sidebarWidth,
      isSidebarCollapsed: isSidebarCollapsed ?? this.isSidebarCollapsed,
    );
  }
}

/// 文件树 UI 状态管理
///
/// 使用 Riverpod 管理文件树的 UI 状态
class FileTreeUIStateNotifier extends Notifier<FileTreeUIState> {
  /// SharedPreferences 实例
  SharedPreferences? _prefs;

  /// 侧边栏宽度存储键
  static const String _sidebarWidthKey = 'file_tree_sidebar_width';

  /// 最小/最大侧边栏宽度
  static const double minSidebarWidth = 200.0;
  static const double maxSidebarWidth = 500.0;

  @override
  FileTreeUIState build() {
    // 异步加载保存的侧边栏宽度
    _loadSidebarWidth();
    return const FileTreeUIState();
  }

  /// 加载保存的侧边栏宽度
  Future<void> _loadSidebarWidth() async {
    _prefs ??= await SharedPreferences.getInstance();
    final savedWidth = _prefs?.getDouble(_sidebarWidthKey);
    if (savedWidth != null) {
      state = state.copyWith(
        sidebarWidth: savedWidth.clamp(minSidebarWidth, maxSidebarWidth),
      );
    }
  }

  /// 切换节点展开/折叠状态
  void toggleExpand(String path) {
    final expandedPaths = Set<String>.from(state.expandedPaths);
    if (expandedPaths.contains(path)) {
      expandedPaths.remove(path);
    } else {
      expandedPaths.add(path);
    }
    state = state.copyWith(expandedPaths: expandedPaths);
  }

  /// 展开指定路径的节点
  void expand(String path) {
    final expandedPaths = Set<String>.from(state.expandedPaths);
    expandedPaths.add(path);
    state = state.copyWith(expandedPaths: expandedPaths);
  }

  /// 折叠指定路径的节点
  void collapse(String path) {
    final expandedPaths = Set<String>.from(state.expandedPaths);
    expandedPaths.remove(path);
    state = state.copyWith(expandedPaths: expandedPaths);
  }

  /// 展开所有节点
  void expandAll() {
    // 注意：这里需要传入完整节点列表才能展开所有节点
    // 暂时只展开根节点
  }

  /// 折叠所有节点
  void collapseAll() {
    state = state.copyWith(expandedPaths: {});
  }

  /// 选中节点（单选）
  void selectNode(String? path) {
    state = state.copyWith(
      selectedPath: path,
      selectedPaths: {},
      clearAnchorPath: true,
    );
  }

  /// 切换选择（Ctrl+点击）
  ///
  /// 如果已选中则取消，否则添加到选中集合
  void toggleSelection(String path) {
    final selectedPaths = Set<String>.from(state.selectedPaths);

    if (selectedPaths.contains(path)) {
      selectedPaths.remove(path);
    } else {
      selectedPaths.add(path);
    }

    state = state.copyWith(
      selectedPaths: selectedPaths,
      selectedPath: path,
      anchorPath: path,
    );
  }

  /// 范围选择（Shift+点击）
  ///
  /// 从锚点路径到目标路径之间的所有节点
  void selectRange(String targetPath, List<String> orderedPaths) {
    final anchorPath = state.anchorPath ?? state.selectedPath ?? targetPath;

    // 找到锚点和目标的索引
    final anchorIndex = orderedPaths.indexOf(anchorPath);
    final targetIndex = orderedPaths.indexOf(targetPath);

    if (anchorIndex == -1 || targetIndex == -1) {
      // 路径不在列表中，直接选中目标
      selectNode(targetPath);
      return;
    }

    // 计算范围
    final start = anchorIndex < targetIndex ? anchorIndex : targetIndex;
    final end = anchorIndex < targetIndex ? targetIndex : anchorIndex;

    // 选中范围内的所有路径
    final selectedPaths = <String>{};
    for (var i = start; i <= end; i++) {
      selectedPaths.add(orderedPaths[i]);
    }

    state = state.copyWith(
      selectedPaths: selectedPaths,
      selectedPath: targetPath,
      anchorPath: anchorPath,
    );
  }

  /// 清除所有选择
  void clearSelection() {
    state = state.copyWith(
      selectedPath: null,
      selectedPaths: {},
      clearSelectedPath: true,
      clearAnchorPath: true,
    );
  }

  /// 检查路径是否被选中
  bool isSelected(String path) {
    return state.selectedPath == path || state.selectedPaths.contains(path);
  }

  /// 设置展开状态（用于从外部更新）
  void setExpandedPaths(Set<String> paths) {
    state = state.copyWith(expandedPaths: paths);
  }

  /// 批量展开路径
  void expandPaths(List<String> paths) {
    final expandedPaths = Set<String>.from(state.expandedPaths);
    expandedPaths.addAll(paths);
    state = state.copyWith(expandedPaths: expandedPaths);
  }

  /// 设置侧边栏宽度
  void setSidebarWidth(double width) {
    final clampedWidth = width.clamp(minSidebarWidth, maxSidebarWidth);
    state = state.copyWith(sidebarWidth: clampedWidth);

    // 保存到本地存储
    _prefs?.setDouble(_sidebarWidthKey, clampedWidth);
  }

  /// 切换侧边栏折叠状态
  void toggleSidebar() {
    state = state.copyWith(isSidebarCollapsed: !state.isSidebarCollapsed);
  }

  /// 展开侧边栏
  void expandSidebar() {
    state = state.copyWith(isSidebarCollapsed: false);
  }

  /// 折叠侧边栏
  void collapseSidebar() {
    state = state.copyWith(isSidebarCollapsed: true);
  }
}

/// 文件树 UI 状态 Provider
final fileTreeUIProvider = NotifierProvider<FileTreeUIStateNotifier, FileTreeUIState>(
  FileTreeUIStateNotifier.new,
);

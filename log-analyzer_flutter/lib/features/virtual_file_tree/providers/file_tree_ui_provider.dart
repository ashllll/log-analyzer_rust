import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:shared_preferences/shared_preferences.dart';

/// 文件树 UI 状态
///
/// 管理展开/折叠状态、选中节点、侧边栏宽度
class FileTreeUIState {
  /// 展开的节点路径集合
  final Set<String> expandedPaths;

  /// 当前选中的节点路径
  final String? selectedPath;

  /// 侧边栏宽度
  final double sidebarWidth;

  /// 侧边栏是否折叠
  final bool isSidebarCollapsed;

  /// 构造函数
  const FileTreeUIState({
    this.expandedPaths = const {},
    this.selectedPath,
    this.sidebarWidth = 280.0,
    this.isSidebarCollapsed = false,
  });

  /// 复制
  FileTreeUIState copyWith({
    Set<String>? expandedPaths,
    String? selectedPath,
    double? sidebarWidth,
    bool? isSidebarCollapsed,
  }) {
    return FileTreeUIState(
      expandedPaths: expandedPaths ?? this.expandedPaths,
      selectedPath: selectedPath ?? this.selectedPath,
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

  /// 选中节点
  void selectNode(String? path) {
    state = state.copyWith(selectedPath: path);
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

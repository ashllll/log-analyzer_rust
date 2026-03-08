import 'package:flutter/foundation.dart';

/// 文件树控制器
///
/// 管理目录展开/折叠状态，提供懒加载触发机制
/// 注意：在 Flutter 3.38+ 中 TreeSliverController 是内部类，
/// 此实现直接管理展开状态，通过通知方式刷新 UI
class FileTreeController extends ChangeNotifier {
  /// 展开的路径集合
  final Set<String> _expandedPaths = {};

  /// 懒加载回调
  final Future<void> Function(String path)? onLoadChildren;

  /// 构造函数
  ///
  /// [onLoadChildren] - 懒加载回调，展开目录时调用
  FileTreeController({this.onLoadChildren});

  /// 获取所有展开的路径
  Set<String> get expandedPaths => Set.unmodifiable(_expandedPaths);

  /// 切换展开/折叠状态
  ///
  /// 如果节点需要懒加载，先加载子节点后再展开
  Future<void> toggleExpansion(String path) async {
    final wasExpanded = _expandedPaths.contains(path);

    if (wasExpanded) {
      // 折叠：直接移除
      _expandedPaths.remove(path);
    } else {
      // 展开：检查是否需要懒加载
      await _handleLazyLoad(path);
      _expandedPaths.add(path);
    }

    notifyListeners();
  }

  /// 处理懒加载
  ///
  /// 如果节点需要懒加载，调用回调加载子节点
  Future<void> _handleLazyLoad(String path) async {
    if (onLoadChildren != null) {
      await onLoadChildren!(path);
    }
  }

  /// 展开指定路径的目录
  ///
  /// 返回是否触发了懒加载
  Future<bool> expand(String path) async {
    if (_expandedPaths.contains(path)) {
      return false; // 已经展开
    }

    await _handleLazyLoad(path);
    _expandedPaths.add(path);
    notifyListeners();
    return true;
  }

  /// 展开指定路径的目录（不触发懒加载）
  ///
  /// 用于已知节点已加载子节点的情况
  void expandWithoutLoading(String path) {
    if (!_expandedPaths.contains(path)) {
      _expandedPaths.add(path);
      notifyListeners();
    }
  }

  /// 折叠指定路径的目录
  void collapse(String path) {
    if (_expandedPaths.contains(path)) {
      _expandedPaths.remove(path);
      notifyListeners();
    }
  }

  /// 检查路径是否展开
  bool isExpanded(String path) {
    return _expandedPaths.contains(path);
  }

  /// 展开所有目录
  ///
  /// [allPaths] - 所有可展开的目录路径列表
  /// 注意：此方法会触发所有节点的懒加载
  Future<void> expandAll(List<String> allPaths) async {
    for (final path in allPaths) {
      await expand(path);
    }
  }

  /// 展开所有目录（不触发懒加载）
  ///
  /// 用于已知所有节点已加载子节点的情况
  void expandAllWithoutLoading(List<String> paths) {
    for (final path in paths) {
      expandWithoutLoading(path);
    }
  }

  /// 折叠所有目录
  void collapseAll() {
    _expandedPaths.clear();
    notifyListeners();
  }

  /// 展开到指定路径
  ///
  /// 递归展开所有父节点
  void expandToPath(String targetPath, List<String> parentPaths) {
    for (final parentPath in parentPaths) {
      if (targetPath.startsWith(parentPath) &&
          !_expandedPaths.contains(parentPath)) {
        expandWithoutLoading(parentPath);
      }
    }
  }

  /// 批量更新展开状态
  ///
  /// 用于从持久化存储恢复状态
  void setExpandedPaths(Set<String> paths) {
    _expandedPaths.clear();
    _expandedPaths.addAll(paths);
    notifyListeners();
  }

  @override
  void dispose() {
    super.dispose();
  }

  @override
  String toString() {
    return 'FileTreeController(expandedPaths: ${_expandedPaths.length} items)';
  }
}

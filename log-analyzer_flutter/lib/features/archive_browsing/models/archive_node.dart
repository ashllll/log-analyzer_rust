import '../../../shared/services/api_service.dart';

/// 压缩包文件树节点
///
/// 用于在树形视图中展示压缩包内的文件结构
class ArchiveNode {
  /// 文件/目录名称
  final String name;

  /// 完整路径
  final String path;

  /// 是否为目录
  final bool isDirectory;

  /// 文件大小（字节）
  final int size;

  /// 压缩后大小（字节）
  final int compressedSize;

  /// 子节点（目录）
  final List<ArchiveNode> children;

  /// 展开状态
  bool isExpanded;

  ArchiveNode({
    required this.name,
    required this.path,
    required this.isDirectory,
    this.size = 0,
    this.compressedSize = 0,
    this.children = const [],
    this.isExpanded = false,
  });

  /// 从 API 返回的 ArchiveEntry 转换
  factory ArchiveNode.fromEntry(ArchiveEntry entry) {
    return ArchiveNode(
      name: entry.name,
      path: entry.path,
      isDirectory: entry.isDirectory,
      size: entry.size,
      compressedSize: 0,
    );
  }

  /// 创建目录节点
  factory ArchiveNode.directory({
    required String name,
    required String path,
    List<ArchiveNode> children = const [],
    bool isExpanded = false,
  }) {
    return ArchiveNode(
      name: name,
      path: path,
      isDirectory: true,
      size: 0,
      compressedSize: 0,
      children: children,
      isExpanded: isExpanded,
    );
  }

  /// 创建文件节点
  factory ArchiveNode.file({
    required String name,
    required String path,
    int size = 0,
    int compressedSize = 0,
  }) {
    return ArchiveNode(
      name: name,
      path: path,
      isDirectory: false,
      size: size,
      compressedSize: compressedSize,
    );
  }

  /// 构建树形结构
  ///
  /// 将扁平的 entries 列表转换为树形结构
  static List<ArchiveNode> buildTree(List<ArchiveEntry> entries) {
    if (entries.isEmpty) return [];

    // 按路径排序，确保父目录在子目录之前处理
    final sortedEntries = List<ArchiveEntry>.from(entries)
      ..sort((a, b) => a.path.compareTo(b.path));

    // 用于存储所有节点
    final nodeMap = <String, ArchiveNode>{};

    // 首先创建所有节点
    for (final entry in sortedEntries) {
      nodeMap[entry.path] = ArchiveNode.fromEntry(entry);
    }

    // 用于跟踪哪些节点已被添加到父节点
    final addedToParent = <String>{};

    // 按路径长度排序，短路径优先（确保父目录先处理）
    sortedEntries.sort((a, b) => a.path.length.compareTo(b.path.length));

    // 建立父子关系
    for (final entry in sortedEntries) {
      final parentPath = _getParentPath(entry.path);

      if (parentPath.isEmpty || !nodeMap.containsKey(parentPath)) {
        // 根级或父目录不存在，不需要额外处理
      } else {
        // 标记这个节点已被处理
        addedToParent.add(entry.path);
      }
    }

    // 构建最终树结构
    final rootNodes = <ArchiveNode>[];

    for (final entry in sortedEntries) {
      final node = nodeMap[entry.path]!;
      final parentPath = _getParentPath(entry.path);

      if (parentPath.isEmpty) {
        // 根节点
        rootNodes.add(node);
      } else {
        final parentNode = nodeMap[parentPath];
        if (parentNode != null && parentNode.isDirectory) {
          // 添加到父节点的 children
          final updatedChildren = List<ArchiveNode>.from(parentNode.children)
            ..add(node);
          nodeMap[parentPath] = parentNode.copyWith(children: updatedChildren);
        } else {
          // 父目录不存在或不是目录，作为根节点
          rootNodes.add(node);
        }
      }
    }

    // 对根节点进行排序（目录在前，文件在后，按名称排序）
    _sortNodes(rootNodes);

    return rootNodes;
  }

  /// 对节点列表进行排序（目录在前，文件在后）
  static void _sortNodes(List<ArchiveNode> nodes) {
    nodes.sort((a, b) {
      if (a.isDirectory && !b.isDirectory) return -1;
      if (!a.isDirectory && b.isDirectory) return 1;
      return a.name.toLowerCase().compareTo(b.name.toLowerCase());
    });

    // 递归排序子节点
    for (final node in nodes) {
      if (node.isDirectory && node.children.isNotEmpty) {
        _sortNodes(node.children);
      }
    }
  }

  /// 获取父目录路径
  static String _getParentPath(String path) {
    final lastSeparator = path.lastIndexOf('/');
    if (lastSeparator <= 0) return '';
    return path.substring(0, lastSeparator);
  }

  /// 复制节点并更新属性
  ArchiveNode copyWith({
    String? name,
    String? path,
    bool? isDirectory,
    int? size,
    int? compressedSize,
    List<ArchiveNode>? children,
    bool? isExpanded,
  }) {
    return ArchiveNode(
      name: name ?? this.name,
      path: path ?? this.path,
      isDirectory: isDirectory ?? this.isDirectory,
      size: size ?? this.size,
      compressedSize: compressedSize ?? this.compressedSize,
      children: children ?? this.children,
      isExpanded: isExpanded ?? this.isExpanded,
    );
  }

  /// 格式化文件大小
  String get formattedSize {
    if (size < 1024) return '$size B';
    if (size < 1024 * 1024) return '${(size / 1024).toStringAsFixed(1)} KB';
    return '${(size / (1024 * 1024)).toStringAsFixed(1)} MB';
  }

  /// 格式化压缩比
  String get compressionRatio {
    if (size == 0 || compressedSize == 0) return '';
    final ratio = (1 - compressedSize / size) * 100;
    return '${ratio.toStringAsFixed(0)}%';
  }

  @override
  String toString() {
    return 'ArchiveNode(name: $name, path: $path, isDirectory: $isDirectory, size: $size)';
  }

  @override
  bool operator ==(Object other) {
    if (identical(this, other)) return true;
    return other is ArchiveNode &&
        other.name == name &&
        other.path == path &&
        other.isDirectory == isDirectory;
  }

  @override
  int get hashCode => name.hashCode ^ path.hashCode ^ isDirectory.hashCode;
}

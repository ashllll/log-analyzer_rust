import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_riverpod/legacy.dart';
import '../models/archive_node.dart';
import '../../../shared/services/api_service.dart';

/// 压缩包路径参数
final archivePathProvider = StateNotifierProvider<ArchivePathNotifier, String?>((ref) {
  return ArchivePathNotifier();
});

class ArchivePathNotifier extends StateNotifier<String?> {
  ArchivePathNotifier() : super(null);

  void setPath(String path) {
    state = path;
  }

  void clear() {
    state = null;
  }
}

/// 压缩包文件树
///
/// 根据压缩包路径加载文件列表并构建树形结构
final archiveTreeProvider =
    FutureProvider.family<List<ArchiveNode>, String>((ref, archivePath) async {
  final api = ApiService();
  final result = await api.listArchiveContents(archivePath);
  return ArchiveNode.buildTree(result.entries);
});

/// 当前选中的文件
final selectedFileProvider = StateNotifierProvider<SelectedFileNotifier, ArchiveNode?>((ref) {
  return SelectedFileNotifier();
});

class SelectedFileNotifier extends StateNotifier<ArchiveNode?> {
  SelectedFileNotifier() : super(null);

  void select(ArchiveNode node) {
    state = node;
  }

  void clear() {
    state = null;
  }
}

/// 搜索关键词
final searchKeywordProvider = StateNotifierProvider<SearchKeywordNotifier, String>((ref) {
  return SearchKeywordNotifier();
});

class SearchKeywordNotifier extends StateNotifier<String> {
  SearchKeywordNotifier() : super('');

  void setKeyword(String keyword) {
    state = keyword;
  }

  void clear() {
    state = '';
  }
}

/// 过滤后的文件列表（用于搜索）
///
/// 根据搜索关键词过滤文件树
final filteredNodesProvider =
    Provider.family<List<ArchiveNode>, List<ArchiveNode>>((ref, nodes) {
  final keyword = ref.watch(searchKeywordProvider);
  if (keyword.isEmpty) return nodes;

  // 过滤逻辑：文件名包含关键词或路径包含关键词
  return _filterNodes(nodes, keyword);
});

/// 加载状态
final isLoadingProvider = StateNotifierProvider<IsLoadingNotifier, bool>((ref) {
  return IsLoadingNotifier();
});

class IsLoadingNotifier extends StateNotifier<bool> {
  IsLoadingNotifier() : super(false);

  void setLoading(bool loading) {
    state = loading;
  }
}

/// 错误信息
final errorProvider = StateNotifierProvider<ErrorNotifier, String?>((ref) {
  return ErrorNotifier();
});

class ErrorNotifier extends StateNotifier<String?> {
  ErrorNotifier() : super(null);

  void setError(String? error) {
    state = error;
  }

  void clear() {
    state = null;
  }
}

/// 辅助类：选中的文件参数
class SelectedFileParams {
  final ArchiveNode? file;
  final String? archivePath;

  SelectedFileParams({this.file, this.archivePath});

  @override
  bool operator ==(Object other) =>
      identical(this, other) ||
      other is SelectedFileParams &&
          runtimeType == other.runtimeType &&
          file == other.file &&
          archivePath == other.archivePath;

  @override
  int get hashCode => file.hashCode ^ archivePath.hashCode;
}

/// 过滤节点
///
/// 递归过滤包含关键词的节点
List<ArchiveNode> _filterNodes(List<ArchiveNode> nodes, String keyword) {
  final result = <ArchiveNode>[];
  final lowerKeyword = keyword.toLowerCase();

  for (final node in nodes) {
    // 文件名或路径包含关键词
    if (node.name.toLowerCase().contains(lowerKeyword) ||
        node.path.toLowerCase().contains(lowerKeyword)) {
      result.add(node);
    } else if (node.isDirectory && node.children.isNotEmpty) {
      // 递归过滤子目录
      final filteredChildren = _filterNodes(node.children, keyword);
      if (filteredChildren.isNotEmpty) {
        result.add(ArchiveNode(
          name: node.name,
          path: node.path,
          isDirectory: node.isDirectory,
          size: node.size,
          compressedSize: node.compressedSize,
          children: filteredChildren,
          isExpanded: true, // 展开包含匹配结果的目录
        ));
      }
    }
  }

  return result;
}

import 'package:flutter/material.dart';
import '../../models/archive_node.dart';

/// 压缩包树形视图组件
class ArchiveTreeView extends StatelessWidget {
  final List<ArchiveNode> nodes;
  final String? selectedPath;
  final void Function(ArchiveNode) onSelect;
  final void Function(ArchiveNode)? onToggleExpand;

  const ArchiveTreeView({
    super.key,
    required this.nodes,
    this.selectedPath,
    required this.onSelect,
    this.onToggleExpand,
  });

  @override
  Widget build(BuildContext context) {
    return ListView.builder(
      itemCount: nodes.length,
      itemBuilder: (context, index) => _buildNode(context, nodes[index], 0),
    );
  }

  Widget _buildNode(BuildContext context, ArchiveNode node, int depth) {
    final isSelected = node.path == selectedPath;

    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        InkWell(
          onTap: () => onSelect(node),
          child: Container(
            padding: EdgeInsets.only(left: depth * 16.0 + 8, top: 8, bottom: 8),
            color: isSelected
                ? Theme.of(context).colorScheme.primaryContainer
                : null,
            child: Row(
              children: [
                // 展开/折叠图标
                if (node.isDirectory)
                  GestureDetector(
                    onTap: () => onToggleExpand?.call(node),
                    child: Icon(
                      node.isExpanded ? Icons.expand_more : Icons.chevron_right,
                      size: 20,
                    ),
                  )
                else
                  const SizedBox(width: 20),
                // 文件/目录图标
                Icon(
                  node.isDirectory ? Icons.folder : _getFileIcon(node.name),
                  size: 20,
                  color: node.isDirectory ? Colors.amber : Colors.grey,
                ),
                const SizedBox(width: 8),
                // 文件名
                Expanded(
                  child: Text(
                    node.name,
                    style: TextStyle(
                      fontWeight: isSelected
                          ? FontWeight.bold
                          : FontWeight.normal,
                    ),
                    overflow: TextOverflow.ellipsis,
                  ),
                ),
                // 文件大小
                if (!node.isDirectory && node.size > 0)
                  Padding(
                    padding: const EdgeInsets.only(left: 8),
                    child: Text(
                      _formatSize(node.size),
                      style: Theme.of(context).textTheme.bodySmall,
                    ),
                  ),
              ],
            ),
          ),
        ),
        // 递归渲染子节点
        if (node.isDirectory && node.isExpanded)
          ...node.children.map(
            (child) => _buildNode(context, child, depth + 1),
          ),
      ],
    );
  }

  /// 获取文件图标
  IconData _getFileIcon(String fileName) {
    final ext = fileName.split('.').last.toLowerCase();
    switch (ext) {
      case 'log':
      case 'txt':
        return Icons.description;
      case 'json':
        return Icons.data_object;
      case 'xml':
        return Icons.code;
      case 'html':
      case 'htm':
        return Icons.html;
      case 'yaml':
      case 'yml':
        return Icons.settings;
      case 'md':
        return Icons.article;
      case 'csv':
        return Icons.table_chart;
      default:
        return Icons.insert_drive_file;
    }
  }

  /// 格式化文件大小
  String _formatSize(int bytes) {
    if (bytes < 1024) return '$bytes B';
    if (bytes < 1024 * 1024) return '${(bytes / 1024).toStringAsFixed(1)} KB';
    return '${(bytes / (1024 * 1024)).toStringAsFixed(1)} MB';
  }
}

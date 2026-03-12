import 'package:flutter/material.dart';
import 'package:lucide_icons_flutter/lucide_icons.dart';

import '../../../../shared/providers/virtual_file_tree_provider.dart';
import 'file_type_icon.dart';

/// 单个文件树节点组件
///
/// 用于展示文件或目录节点，支持展开/折叠、选中状态
class FileTreeNode extends StatelessWidget {
  /// 节点数据
  final VirtualTreeNode node;

  /// 是否展开
  final bool isExpanded;

  /// 是否选中
  final bool isSelected;

  /// 点击回调
  final VoidCallback? onTap;

  /// 展开/折叠回调
  final VoidCallback? onExpand;

  /// 缩进深度（用于显示层级）
  final int depth;

  /// 构造函数
  const FileTreeNode({
    super.key,
    required this.node,
    this.isExpanded = false,
    this.isSelected = false,
    this.onTap,
    this.onExpand,
    this.depth = 0,
  });

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final isDirectory = node.isArchive;

    // 计算背景颜色
    final Color? backgroundColor;
    if (isSelected) {
      backgroundColor = theme.colorScheme.primaryContainer;
    } else {
      backgroundColor = null;
    }

    return Material(
      color: backgroundColor ?? Colors.transparent,
      child: InkWell(
        onTap: onTap,
        hoverColor: theme.colorScheme.surfaceContainerHighest.withOpacity(
          0.5,
        ),
        child: Container(
          height: 28, // 紧凑模式行高
          padding: EdgeInsets.only(left: depth * 16.0 + 4, right: 8),
          child: Row(
            children: [
              // 展开/折叠箭头（仅目录显示）
              SizedBox(
                width: 20,
                height: 20,
                child: isDirectory
                    ? GestureDetector(
                        onTap: onExpand,
                        child: Icon(
                          isExpanded
                              ? LucideIcons.chevronDown
                              : LucideIcons.chevronRight,
                          size: 16,
                          color: theme.colorScheme.onSurfaceVariant,
                        ),
                      )
                    : null,
              ),
              const SizedBox(width: 4),
              // 文件/目录图标
              Icon(
                isDirectory
                    ? (isExpanded ? directoryOpenIcon : directoryIcon)
                    : getFileIcon(node.nodeName),
                size: 18,
                color: isDirectory
                    ? const Color(0xFFFFB74D) // 琥珀色目录图标
                    : getFileIconColor(node.nodeName),
              ),
              const SizedBox(width: 8),
              // 文件名（带 tooltip）
              Expanded(
                child: Tooltip(
                  message: node.nodePath,
                  waitDuration: const Duration(milliseconds: 500),
                  child: Text(
                    node.nodeName,
                    style: TextStyle(
                      fontSize: 13,
                      fontWeight: isSelected
                          ? FontWeight.w600
                          : FontWeight.normal,
                      color: isSelected
                          ? theme.colorScheme.onPrimaryContainer
                          : theme.colorScheme.onSurface,
                    ),
                    overflow: TextOverflow.ellipsis,
                    maxLines: 1,
                  ),
                ),
              ),
            ],
          ),
        ),
      ),
    );
  }
}

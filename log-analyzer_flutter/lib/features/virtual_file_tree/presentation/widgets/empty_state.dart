import 'package:flutter/material.dart';
import 'package:lucide_icons_flutter/lucide_icons.dart';

/// 虚拟文件树空状态组件
///
/// 当工作区为空时显示友好提示和导入按钮
/// 支持无障碍访问
class VirtualFileTreeEmptyState extends StatelessWidget {
  /// 导入按钮点击回调
  final VoidCallback? onImport;

  /// 构造函数
  const VirtualFileTreeEmptyState({super.key, this.onImport});

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);

    return Semantics(
      label: '工作区为空，导入文件开始分析',
      child: Center(
        child: Padding(
          padding: const EdgeInsets.all(32),
          child: Column(
            mainAxisAlignment: MainAxisAlignment.center,
            children: [
              // 文件夹图标
              Icon(
                LucideIcons.folderOpen,
                size: 80,
                color: theme.colorScheme.outline.withValues(alpha: 0.5),
              ),
              const SizedBox(height: 24),
              // 标题
              Text(
                '工作区为空',
                style: theme.textTheme.titleLarge?.copyWith(
                  color: theme.colorScheme.onSurface,
                ),
              ),
              const SizedBox(height: 8),
              // 描述
              Text(
                '导入文件开始分析',
                style: theme.textTheme.bodyMedium?.copyWith(
                  color: theme.colorScheme.onSurfaceVariant,
                ),
              ),
              const SizedBox(height: 24),
              // 导入按钮
              if (onImport != null)
                Semantics(
                  button: true,
                  label: '导入文件',
                  child: FilledButton.icon(
                    onPressed: onImport,
                    icon: const Icon(LucideIcons.import),
                    label: const Text('导入文件'),
                  ),
                ),
            ],
          ),
        ),
      ),
    );
  }
}

/// 文件预览空状态组件
///
/// 当没有选中文件时显示提示
/// 支持无障碍访问
class FilePreviewEmptyState extends StatelessWidget {
  const FilePreviewEmptyState({super.key});

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);

    return Semantics(
      label: '文件预览为空，请选择文件',
      child: Center(
        child: Column(
          mainAxisAlignment: MainAxisAlignment.center,
          children: [
            Icon(LucideIcons.file, size: 48, color: theme.colorScheme.outline),
            const SizedBox(height: 16),
            Text(
              '选择文件预览内容',
              style: theme.textTheme.bodyMedium?.copyWith(
                color: theme.colorScheme.outline,
              ),
            ),
          ],
        ),
      ),
    );
  }
}

import 'package:flutter/material.dart';
import 'package:desktop_drop/desktop_drop.dart';
import '../../core/theme/app_theme.dart';
import '../services/api_service.dart';

/// 拖放区域回调函数类型
typedef OnFilesDropped = void Function(List<String> paths);

/// 压缩包拖放回调函数类型
typedef OnArchiveDropped = void Function(String archivePath);

/// 拖放区域组件
///
/// 使用 desktop_drop 包实现拖放功能
/// 支持拖入文件夹/文件/压缩包，显示视觉反馈
class DropZoneWidget extends StatefulWidget {
  /// 子组件
  final Widget child;

  /// 拖放完成回调
  final OnFilesDropped? onFilesDropped;

  /// 压缩包拖放完成回调
  final OnArchiveDropped? onArchiveDropped;

  /// 是否启用拖放
  final bool enabled;

  /// 拖放区域边框样式
  final BoxBorder? border;

  /// 拖放区域背景色
  final Color? backgroundColor;

  /// 拖入时的高亮边框颜色
  final Color? highlightColor;

  /// 允许的文件扩展名 (如 ['.log', '.txt', '.json'])
  final List<String>? allowedExtensions;

  /// 是否仅接受文件夹
  final bool foldersOnly;

  /// 是否启用压缩包支持
  final bool archiveEnabled;

  const DropZoneWidget({
    super.key,
    required this.child,
    this.onFilesDropped,
    this.onArchiveDropped,
    this.enabled = true,
    this.border,
    this.backgroundColor,
    this.highlightColor,
    this.allowedExtensions,
    this.foldersOnly = false,
    this.archiveEnabled = true,
  });

  @override
  State<DropZoneWidget> createState() => _DropZoneWidgetState();
}

class _DropZoneWidgetState extends State<DropZoneWidget> {
  bool _isDragging = false;

  @override
  Widget build(BuildContext context) {
    if (!widget.enabled) {
      return widget.child;
    }

    return DropTarget(
      onDragEntered: (details) {
        setState(() {
          _isDragging = true;
        });
      },
      onDragExited: (details) {
        setState(() {
          _isDragging = false;
        });
      },
      onDragDone: (details) {
        setState(() {
          _isDragging = false;
        });

        // 分类处理拖放的文件
        final result = _classifyFiles(details.files);

        // 如果启用了压缩包支持且有压缩包
        if (widget.archiveEnabled && widget.onArchiveDropped != null) {
          // 如果有压缩包，优先处理压缩包（显示预览对话框）
          if (result['archives'] != null && result['archives']!.isNotEmpty) {
            // 取第一个压缩包，显示预览
            widget.onArchiveDropped!(result['archives']!.first);
            return;
          }
        }

        // 处理普通文件和文件夹
        final paths = <String>[];
        if (result['files'] != null) {
          paths.addAll(result['files']!);
        }
        if (result['folders'] != null) {
          paths.addAll(result['folders']!);
        }

        if (paths.isNotEmpty && widget.onFilesDropped != null) {
          widget.onFilesDropped!(paths);
        }
      },
      child: AnimatedContainer(
        duration: const Duration(milliseconds: 200),
        decoration: BoxDecoration(
          border: widget.border ?? (_isDragging
              ? Border.all(
                  color: widget.highlightColor ?? AppColors.primary,
                  width: 2,
                )
              : null),
          color: _isDragging
              ? (widget.backgroundColor ?? AppColors.primary).withValues(alpha: 0.1)
              : widget.backgroundColor,
          borderRadius: BorderRadius.circular(8),
        ),
        child: Stack(
          children: [
            widget.child,
            // 拖入时的覆盖层
            if (_isDragging)
              Positioned.fill(
                child: Container(
                  decoration: BoxDecoration(
                    color: AppColors.primary.withValues(alpha: 0.1),
                    borderRadius: BorderRadius.circular(8),
                    border: Border.all(
                      color: AppColors.primary,
                      width: 2,
                      style: BorderStyle.solid,
                    ),
                  ),
                  child: Center(
                    child: Column(
                      mainAxisSize: MainAxisSize.min,
                      children: [
                        Icon(
                          Icons.file_download,
                          size: 48,
                          color: AppColors.primary,
                        ),
                        const SizedBox(height: 8),
                        Text(
                          '释放以导入文件',
                          style: TextStyle(
                            color: AppColors.primary,
                            fontSize: 16,
                            fontWeight: FontWeight.w500,
                          ),
                        ),
                        const SizedBox(height: 4),
                        Text(
                          _getDropHintText(),
                          style: TextStyle(
                            color: AppColors.textSecondary,
                            fontSize: 12,
                          ),
                        ),
                      ],
                    ),
                  ),
                ),
              ),
          ],
        ),
      ),
    );
  }

  /// 分类处理拖放的文件
  ///
  /// 返回包含 archives、files、folders 的 Map
  Map<String, List<String>> _classifyFiles(List<XFile> files) {
    final archives = <String>[];
    final regularFiles = <String>[];
    final folders = <String>[];

    for (final file in files) {
      final path = file.path;

      // 检查是否为压缩包
      if (widget.archiveEnabled && ApiService.isArchiveFile(path)) {
        archives.add(path);
        continue;
      }

      // 检查是否为文件夹
      if (_isFolder(path)) {
        folders.add(path);
        continue;
      }

      // 检查是否为允许的文件类型
      if (widget.foldersOnly) {
        // 如果仅接受文件夹，跳过文件
        continue;
      } else if (widget.allowedExtensions != null &&
          widget.allowedExtensions!.isNotEmpty) {
        // 检查文件扩展名
        final extension = _getExtension(path);
        if (!widget.allowedExtensions!.any(
          (ext) => ext.toLowerCase() == extension.toLowerCase(),
        )) {
          continue;
        }
      }

      regularFiles.add(path);
    }

    return {
      'archives': archives,
      'files': regularFiles,
      'folders': folders,
    };
  }

  /// 获取文件扩展名
  String _getExtension(String path) {
    final lastDot = path.lastIndexOf('.');
    if (lastDot == -1) return '';
    return path.substring(lastDot);
  }

  /// 简单判断是否为文件夹 (通过检查路径特征)
  /// 注意: desktop_drop 在某些平台上可能无法准确判断
  bool _isFolder(String path) {
    // Windows: 检查路径是否以反斜杠结尾或没有扩展名
    if (path.endsWith('\\') || path.endsWith('/')) {
      return true;
    }
    // 没有文件扩展名的可能是文件夹
    return !path.contains('.') || path.endsWith('/') || path.endsWith('\\');
  }

  /// 获取拖放提示文本
  String _getDropHintText() {
    if (widget.foldersOnly) {
      return '仅接受文件夹';
    }

    final parts = <String>[];

    if (widget.archiveEnabled) {
      parts.add('ZIP/TAR/GZ/RAR/7Z');
    }

    if (widget.allowedExtensions != null && widget.allowedExtensions!.isNotEmpty) {
      parts.add(widget.allowedExtensions!.join(', '));
    }

    if (parts.isEmpty) {
      return '支持文件和文件夹';
    }

    return parts.join(' | ');
  }
}

/// 拖放提示组件
///
/// 在拖放区域旁边显示拖放提示
class DropZoneHint extends StatelessWidget {
  final String? title;
  final String? subtitle;
  final IconData? icon;

  const DropZoneHint({
    super.key,
    this.title,
    this.subtitle,
    this.icon,
  });

  @override
  Widget build(BuildContext context) {
    return Column(
      mainAxisSize: MainAxisSize.min,
      children: [
        Icon(
          icon ?? Icons.upload_file,
          size: 32,
          color: AppColors.textMuted,
        ),
        if (title != null) ...[
          const SizedBox(height: 8),
          Text(
            title!,
            style: const TextStyle(
              color: AppColors.textSecondary,
              fontSize: 14,
              fontWeight: FontWeight.w500,
            ),
          ),
        ],
        if (subtitle != null) ...[
          const SizedBox(height: 4),
          Text(
            subtitle!,
            style: const TextStyle(
              color: AppColors.textMuted,
              fontSize: 12,
            ),
          ),
        ],
      ],
    );
  }
}

/// 简化版拖放区域
///
/// 用于快速集成到现有组件中
class SimpleDropZone extends StatelessWidget {
  final OnFilesDropped onFilesDropped;
  final OnArchiveDropped? onArchiveDropped;
  final Widget child;
  final bool foldersOnly;

  const SimpleDropZone({
    super.key,
    required this.onFilesDropped,
    this.onArchiveDropped,
    required this.child,
    this.foldersOnly = false,
  });

  @override
  Widget build(BuildContext context) {
    return DropZoneWidget(
      onFilesDropped: onFilesDropped,
      onArchiveDropped: onArchiveDropped,
      foldersOnly: foldersOnly,
      child: child,
    );
  }
}

import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import '../services/api_service.dart';
import '../../core/theme/app_theme.dart';

/// 压缩包导入对话框组件
///
/// 显示压缩包内容列表，支持选择要导入的文件
/// 集成导入进度显示
class ArchiveImportDialog extends ConsumerStatefulWidget {
  /// 压缩包文件路径
  final String archivePath;

  /// 工作区 ID
  final String workspaceId;

  /// 导入完成回调
  final void Function(String taskId)? onImportComplete;

  /// 取消回调
  final VoidCallback? onCancel;

  const ArchiveImportDialog({
    super.key,
    required this.archivePath,
    required this.workspaceId,
    this.onImportComplete,
    this.onCancel,
  });

  /// 显示压缩包导入对话框
  ///
  /// 返回导入任务 ID 或 null（如果取消）
  static Future<String?> show(
    BuildContext context, {
    required String archivePath,
    required String workspaceId,
  }) {
    return showDialog<String>(
      context: context,
      barrierDismissible: false,
      builder: (context) => ArchiveImportDialog(
        archivePath: archivePath,
        workspaceId: workspaceId,
      ),
    );
  }

  @override
  ConsumerState<ArchiveImportDialog> createState() =>
      _ArchiveImportDialogState();
}

class _ArchiveImportDialogState extends ConsumerState<ArchiveImportDialog> {
  /// 加载状态
  bool _isLoading = true;

  /// 压缩包内容
  ArchiveContents? _contents;

  /// 选中的文件
  final Set<String> _selectedFiles = {};

  /// 错误信息
  String? _error;

  /// 导入状态
  bool _isImporting = false;

  /// 导入进度 (0-100)
  double _progress = 0;

  /// 当前正在处理的文件
  String _currentFile = '';

  @override
  void initState() {
    super.initState();
    _loadArchiveContents();
  }

  /// 加载压缩包内容
  Future<void> _loadArchiveContents() async {
    try {
      // 尝试获取压缩包内容（当前为模拟实现）
      final contents = await ApiService().listArchiveContents(
        widget.archivePath,
      );
      setState(() {
        _contents = contents;
        _isLoading = false;
        // 默认选中所有文件
        for (final entry in contents.entries) {
          if (!entry.isDirectory) {
            _selectedFiles.add(entry.path);
          }
        }
      });
    } catch (e) {
      setState(() {
        _error = e.toString();
        _isLoading = false;
      });
    }
  }

  /// 切换文件选择
  void _toggleFile(String path) {
    setState(() {
      if (_selectedFiles.contains(path)) {
        _selectedFiles.remove(path);
      } else {
        _selectedFiles.add(path);
      }
    });
  }

  /// 全选
  void _selectAll() {
    setState(() {
      if (_contents != null) {
        for (final entry in _contents!.entries) {
          if (!entry.isDirectory) {
            _selectedFiles.add(entry.path);
          }
        }
      }
    });
  }

  /// 取消全选
  void _deselectAll() {
    setState(() {
      _selectedFiles.clear();
    });
  }

  /// 开始导入
  Future<void> _startImport() async {
    setState(() {
      _isImporting = true;
      _progress = 0;
      _currentFile = '正在启动导入...';
    });

    try {
      // 调用导入 API
      final taskId = await ApiService().importArchiveFiles(
        archivePath: widget.archivePath,
        workspaceId: widget.workspaceId,
        selectedFiles: _selectedFiles.toList(),
      );

      setState(() {
        _progress = 100;
        _currentFile = '导入完成';
      });

      // 延迟关闭对话框，让用户看到完成状态
      await Future.delayed(const Duration(milliseconds: 500));

      if (mounted) {
        widget.onImportComplete?.call(taskId);
        Navigator.of(context).pop(taskId);
      }
    } catch (e) {
      setState(() {
        _error = e.toString();
        _isImporting = false;
      });
    }
  }

  /// 格式化文件大小
  String _formatSize(int bytes) {
    if (bytes < 1024) return '$bytes B';
    if (bytes < 1024 * 1024) return '${(bytes / 1024).toStringAsFixed(1)} KB';
    if (bytes < 1024 * 1024 * 1024) {
      return '${(bytes / (1024 * 1024)).toStringAsFixed(1)} MB';
    }
    return '${(bytes / (1024 * 1024 * 1024)).toStringAsFixed(2)} GB';
  }

  /// 获取压缩包类型图标
  IconData _getArchiveTypeIcon(ArchiveType? type) {
    switch (type) {
      case ArchiveType.zip:
        return Icons.folder_zip;
      case ArchiveType.tar:
        return Icons.inventory_2;
      case ArchiveType.gzip:
        return Icons.compress;
      case ArchiveType.rar:
        return Icons.lock;
      case ArchiveType.sevenZ:
        return Icons.archive;
      default:
        return Icons.archive;
    }
  }

  /// 获取压缩包类型名称
  String _getArchiveTypeName(ArchiveType? type) {
    switch (type) {
      case ArchiveType.zip:
        return 'ZIP 压缩包';
      case ArchiveType.tar:
        return 'TAR 压缩包';
      case ArchiveType.gzip:
        return 'GZIP 压缩包';
      case ArchiveType.rar:
        return 'RAR 压缩包';
      case ArchiveType.sevenZ:
        return '7Z 压缩包';
      default:
        return '未知格式';
    }
  }

  @override
  Widget build(BuildContext context) {
    // 获取文件名
    final fileName = widget.archivePath.split(RegExp(r'[/\\]')).last;

    return AlertDialog(
      backgroundColor: AppColors.bgCard,
      title: Row(
        children: [
          Icon(_getArchiveTypeIcon(_contents?.type), color: AppColors.primary),
          const SizedBox(width: 8),
          Expanded(
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Text(
                  _getArchiveTypeName(_contents?.type),
                  style: const TextStyle(fontSize: 16),
                ),
                Text(
                  fileName,
                  style: const TextStyle(
                    fontSize: 12,
                    color: AppColors.textMuted,
                    fontWeight: FontWeight.normal,
                  ),
                  maxLines: 1,
                  overflow: TextOverflow.ellipsis,
                ),
              ],
            ),
          ),
        ],
      ),
      content: SizedBox(width: 500, height: 400, child: _buildContent()),
      actions: _buildActions(),
    );
  }

  /// 构建内容区域
  Widget _buildContent() {
    if (_isLoading) {
      return const Center(
        child: Column(
          mainAxisSize: MainAxisSize.min,
          children: [
            CircularProgressIndicator(),
            SizedBox(height: 16),
            Text(
              '正在读取压缩包内容...',
              style: TextStyle(color: AppColors.textSecondary),
            ),
          ],
        ),
      );
    }

    if (_error != null) {
      return Center(
        child: Column(
          mainAxisSize: MainAxisSize.min,
          children: [
            const Icon(Icons.error_outline, color: AppColors.error, size: 48),
            const SizedBox(height: 16),
            Text(
              '读取失败: $_error',
              style: const TextStyle(color: AppColors.error),
              textAlign: TextAlign.center,
            ),
          ],
        ),
      );
    }

    if (_contents == null || _contents!.entries.isEmpty) {
      return _buildEmptyState();
    }

    return Column(
      children: [
        // 工具栏
        _buildToolbar(),
        const SizedBox(height: 8),
        // 文件列表
        Expanded(child: _buildFileList()),
        const SizedBox(height: 8),
        // 底部信息
        _buildBottomInfo(),
      ],
    );
  }

  /// 构建工具栏
  Widget _buildToolbar() {
    return Row(
      children: [
        // 全选按钮
        TextButton.icon(
          onPressed: _selectedFiles.length == _contents?.entries.length
              ? _deselectAll
              : _selectAll,
          icon: Icon(
            _selectedFiles.length == _contents?.entries.length
                ? Icons.check_box
                : Icons.check_box_outline_blank,
            size: 18,
          ),
          label: Text(
            _selectedFiles.length == _contents?.entries.length ? '取消全选' : '全选',
          ),
        ),
        const Spacer(),
        // 搜索框（预留）
        // TODO: 添加搜索功能
      ],
    );
  }

  /// 构建文件列表
  Widget _buildFileList() {
    final entries = _contents!.entries;

    return Container(
      decoration: BoxDecoration(
        color: AppColors.bgMain,
        borderRadius: BorderRadius.circular(8),
      ),
      child: ListView.builder(
        itemCount: entries.length,
        itemBuilder: (context, index) {
          final entry = entries[index];
          final isSelected = _selectedFiles.contains(entry.path);

          return InkWell(
            onTap: entry.isDirectory ? null : () => _toggleFile(entry.path),
            child: Container(
              padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 8),
              decoration: BoxDecoration(
                color: isSelected
                    ? AppColors.primary.withValues(alpha: 0.1)
                    : null,
                border: const Border(
                  bottom: BorderSide(color: AppColors.bgCard, width: 1),
                ),
              ),
              child: Row(
                children: [
                  // 选择框
                  if (!entry.isDirectory)
                    Checkbox(
                      value: isSelected,
                      onChanged: (value) => _toggleFile(entry.path),
                      activeColor: AppColors.primary,
                    )
                  else
                    const SizedBox(width: 40),
                  const SizedBox(width: 8),
                  // 文件/文件夹图标
                  Icon(
                    entry.isDirectory ? Icons.folder : Icons.insert_drive_file,
                    size: 20,
                    color: entry.isDirectory
                        ? AppColors.warning
                        : AppColors.textSecondary,
                  ),
                  const SizedBox(width: 8),
                  // 文件名
                  Expanded(
                    child: Text(
                      entry.name,
                      style: TextStyle(
                        fontSize: 13,
                        color: entry.isDirectory
                            ? AppColors.textPrimary
                            : AppColors.textSecondary,
                      ),
                      maxLines: 1,
                      overflow: TextOverflow.ellipsis,
                    ),
                  ),
                  const SizedBox(width: 8),
                  // 文件大小
                  if (!entry.isDirectory)
                    Text(
                      _formatSize(entry.size),
                      style: const TextStyle(
                        fontSize: 12,
                        color: AppColors.textMuted,
                      ),
                    ),
                ],
              ),
            ),
          );
        },
      ),
    );
  }

  /// 构建底部信息
  Widget _buildBottomInfo() {
    final totalSize = _contents!.entries
        .where((e) => _selectedFiles.contains(e.path))
        .fold<int>(0, (sum, e) => sum + e.size);

    return Row(
      children: [
        Text(
          '已选择 ${_selectedFiles.length} / ${_contents!.entries.length} 个文件',
          style: const TextStyle(fontSize: 12, color: AppColors.textSecondary),
        ),
        const Spacer(),
        Text(
          '预估大小: ${_formatSize(totalSize)}',
          style: const TextStyle(fontSize: 12, color: AppColors.textMuted),
        ),
      ],
    );
  }

  /// 构建空状态
  Widget _buildEmptyState() {
    return const Center(
      child: Column(
        mainAxisSize: MainAxisSize.min,
        children: [
          Icon(Icons.folder_open, size: 64, color: AppColors.textMuted),
          SizedBox(height: 16),
          Text(
            '压缩包为空',
            style: TextStyle(fontSize: 16, color: AppColors.textSecondary),
          ),
          SizedBox(height: 8),
          Text(
            '该压缩包不包含任何文件',
            style: TextStyle(fontSize: 12, color: AppColors.textMuted),
          ),
        ],
      ),
    );
  }

  /// 构建操作按钮
  List<Widget> _buildActions() {
    if (_isImporting) {
      return [
        // 导入中显示进度
        SizedBox(
          width: 200,
          child: Column(
            mainAxisSize: MainAxisSize.min,
            children: [
              LinearProgressIndicator(
                value: _progress / 100,
                backgroundColor: AppColors.bgMain,
                valueColor: const AlwaysStoppedAnimation<Color>(
                  AppColors.primary,
                ),
              ),
              const SizedBox(height: 4),
              Text(
                _currentFile,
                style: const TextStyle(
                  fontSize: 11,
                  color: AppColors.textMuted,
                ),
                maxLines: 1,
                overflow: TextOverflow.ellipsis,
              ),
            ],
          ),
        ),
      ];
    }

    return [
      // 取消按钮
      TextButton(
        onPressed: () {
          widget.onCancel?.call();
          Navigator.of(context).pop();
        },
        child: const Text('取消'),
      ),
      // 导入按钮
      ElevatedButton(
        onPressed: _selectedFiles.isEmpty ? null : _startImport,
        child: Text('导入 ${_selectedFiles.length} 个文件'),
      ),
    ];
  }
}

/// 显示压缩包导入预览对话框
///
/// 这是一个便捷函数，用于快速显示压缩包导入预览
Future<String?> showArchiveImportDialog(
  BuildContext context, {
  required String archivePath,
  required String workspaceId,
}) {
  return ArchiveImportDialog.show(
    context,
    archivePath: archivePath,
    workspaceId: workspaceId,
  );
}

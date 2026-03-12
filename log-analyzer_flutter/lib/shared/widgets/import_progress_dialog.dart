import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import '../providers/import_progress_provider.dart';
import '../../core/theme/app_theme.dart';

/// 导入进度对话框组件
///
/// 显示导入进度、当前文件、预估时间等信息
/// 支持取消操作
class ImportProgressDialog extends ConsumerWidget {
  /// 是否模态对话框
  final bool isModal;

  /// 对话框关闭回调
  final VoidCallback? onClose;

  const ImportProgressDialog({super.key, this.isModal = true, this.onClose});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final state = ref.watch(importProgressProvider);

    if (isModal) {
      return _buildModalDialog(context, ref, state);
    } else {
      return _buildEmbeddedWidget(ref, state);
    }
  }

  /// 构建模态对话框
  Widget _buildModalDialog(
    BuildContext context,
    WidgetRef ref,
    ImportProgressState state,
  ) {
    return AlertDialog(
      backgroundColor: AppColors.bgCard,
      title: Row(
        children: [
          _buildStatusIcon(state.status),
          const SizedBox(width: 8),
          Text(_getTitle(state.status)),
        ],
      ),
      content: SizedBox(width: 400, child: _buildContent(ref, state)),
      actions: _buildActions(context, ref, state),
    );
  }

  /// 构建内嵌组件
  Widget _buildEmbeddedWidget(WidgetRef ref, ImportProgressState state) {
    return Container(
      padding: const EdgeInsets.all(16),
      decoration: BoxDecoration(
        color: AppColors.bgCard,
        borderRadius: BorderRadius.circular(8),
      ),
      child: Column(
        mainAxisSize: MainAxisSize.min,
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Row(
            children: [
              _buildStatusIcon(state.status),
              const SizedBox(width: 8),
              Text(
                _getTitle(state.status),
                style: const TextStyle(
                  fontSize: 16,
                  fontWeight: FontWeight.w600,
                ),
              ),
            ],
          ),
          const SizedBox(height: 16),
          _buildContent(ref, state),
        ],
      ),
    );
  }

  /// 构建内容区域
  Widget _buildContent(WidgetRef ref, ImportProgressState state) {
    return Column(
      mainAxisSize: MainAxisSize.min,
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        // 进度条
        _buildProgressSection(state),
        const SizedBox(height: 16),
        // 当前文件
        if (state.currentFile.isNotEmpty) ...[
          _buildCurrentFile(state),
          const SizedBox(height: 8),
        ],
        // 统计信息
        _buildStatistics(state),
        // 错误列表
        if (state.errors.isNotEmpty) ...[
          const SizedBox(height: 16),
          _buildErrors(state),
        ],
      ],
    );
  }

  /// 构建进度部分
  Widget _buildProgressSection(ImportProgressState state) {
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        // 圆形进度指示器 (仅在导入中显示)
        if (state.status == ImportStatus.importing ||
            state.status == ImportStatus.paused)
          Center(
            child: Stack(
              alignment: Alignment.center,
              children: [
                SizedBox(
                  width: 80,
                  height: 80,
                  child: CircularProgressIndicator(
                    value: state.progressPercent,
                    strokeWidth: 6,
                    backgroundColor: AppColors.bgMain,
                    valueColor: AlwaysStoppedAnimation<Color>(
                      state.status == ImportStatus.paused
                          ? AppColors.warning
                          : AppColors.primary,
                    ),
                  ),
                ),
                Text(
                  '${(state.progressPercent * 100).round()}%',
                  style: const TextStyle(
                    fontSize: 16,
                    fontWeight: FontWeight.w600,
                  ),
                ),
              ],
            ),
          ),
        // 线性进度条 (总是显示)
        if (state.status != ImportStatus.idle)
          Padding(
            padding: const EdgeInsets.only(top: 8),
            child: LinearProgressIndicator(
              value: state.progressPercent,
              backgroundColor: AppColors.bgMain,
              valueColor: AlwaysStoppedAnimation<Color>(
                _getProgressColor(state.status),
              ),
            ),
          ),
      ],
    );
  }

  /// 构建当前文件显示
  Widget _buildCurrentFile(ImportProgressState state) {
    return Row(
      children: [
        const Icon(
          Icons.insert_drive_file,
          size: 16,
          color: AppColors.textMuted,
        ),
        const SizedBox(width: 8),
        Expanded(
          child: Text(
            state.currentFile,
            style: const TextStyle(
              fontSize: 13,
              color: AppColors.textSecondary,
            ),
            maxLines: 1,
            overflow: TextOverflow.ellipsis,
          ),
        ),
      ],
    );
  }

  /// 构建统计信息
  Widget _buildStatistics(ImportProgressState state) {
    return Row(
      children: [
        // 已处理文件数
        Expanded(
          child: _buildStatItem(
            icon: Icons.check_circle_outline,
            label: '已处理',
            value: '${state.processedFiles} / ${state.totalFiles}',
          ),
        ),
        // 处理速度
        if (state.filesPerSecond != null)
          Expanded(
            child: _buildStatItem(
              icon: Icons.speed,
              label: '速度',
              value: '${state.filesPerSecond!.toStringAsFixed(1)} 文件/秒',
            ),
          ),
        // 预估剩余时间
        if (state.estimatedRemainingSeconds != null)
          Expanded(
            child: _buildStatItem(
              icon: Icons.timer,
              label: '剩余',
              value: _formatTime(state.estimatedRemainingSeconds!),
            ),
          ),
      ],
    );
  }

  /// 构建统计项
  Widget _buildStatItem({
    required IconData icon,
    required String label,
    required String value,
  }) {
    return Column(
      children: [
        Icon(icon, size: 16, color: AppColors.textMuted),
        const SizedBox(height: 4),
        Text(
          value,
          style: const TextStyle(fontSize: 12, fontWeight: FontWeight.w500),
        ),
        Text(
          label,
          style: const TextStyle(fontSize: 10, color: AppColors.textMuted),
        ),
      ],
    );
  }

  /// 构建错误列表
  Widget _buildErrors(ImportProgressState state) {
    return Container(
      padding: const EdgeInsets.all(8),
      decoration: BoxDecoration(
        color: AppColors.error.withOpacity(0.1),
        borderRadius: BorderRadius.circular(4),
        border: Border.all(color: AppColors.error.withOpacity(0.3)),
      ),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Row(
            children: [
              const Icon(Icons.error_outline, size: 16, color: AppColors.error),
              const SizedBox(width: 4),
              Text(
                '错误 (${state.errors.length})',
                style: const TextStyle(
                  fontSize: 12,
                  fontWeight: FontWeight.w500,
                  color: AppColors.error,
                ),
              ),
            ],
          ),
          const SizedBox(height: 4),
          ...state.errors
              .take(5)
              .map(
                (error) => Padding(
                  padding: const EdgeInsets.only(left: 20, top: 2),
                  child: Text(
                    error,
                    style: const TextStyle(
                      fontSize: 11,
                      color: AppColors.textSecondary,
                    ),
                    maxLines: 1,
                    overflow: TextOverflow.ellipsis,
                  ),
                ),
              ),
          if (state.errors.length > 5)
            Padding(
              padding: const EdgeInsets.only(left: 20, top: 2),
              child: Text(
                '... 还有 ${state.errors.length - 5} 个错误',
                style: const TextStyle(fontSize: 11, color: AppColors.textMuted),
              ),
            ),
        ],
      ),
    );
  }

  /// 构建操作按钮
  List<Widget> _buildActions(
    BuildContext context,
    WidgetRef ref,
    ImportProgressState state,
  ) {
    final actions = <Widget>[];

    switch (state.status) {
      case ImportStatus.importing:
      case ImportStatus.paused:
        // 取消按钮
        actions.add(
          TextButton(
            onPressed: () {
              ref.read(importProgressProvider.notifier).cancelImport();
            },
            child: const Text('取消'),
          ),
        );
        // 暂停/继续按钮
        if (state.status == ImportStatus.importing) {
          actions.add(
            TextButton(
              onPressed: () {
                ref.read(importProgressProvider.notifier).pauseImport();
              },
              child: const Text('暂停'),
            ),
          );
        } else {
          actions.add(
            ElevatedButton(
              onPressed: () {
                ref.read(importProgressProvider.notifier).resumeImport();
              },
              child: const Text('继续'),
            ),
          );
        }
        break;

      case ImportStatus.completed:
      case ImportStatus.cancelled:
      case ImportStatus.failed:
        // 关闭按钮
        actions.add(
          ElevatedButton(
            onPressed: () {
              ref.read(importProgressProvider.notifier).reset();
              Navigator.of(context).pop();
              onClose?.call();
            },
            child: const Text('关闭'),
          ),
        );
        break;

      case ImportStatus.idle:
        // 无操作
        break;
    }

    return actions;
  }

  /// 获取标题
  String _getTitle(ImportStatus status) {
    switch (status) {
      case ImportStatus.idle:
        return '导入进度';
      case ImportStatus.importing:
        return '正在导入...';
      case ImportStatus.paused:
        return '已暂停';
      case ImportStatus.completed:
        return '导入完成';
      case ImportStatus.cancelled:
        return '已取消';
      case ImportStatus.failed:
        return '导入失败';
    }
  }

  /// 获取状态图标
  Widget _buildStatusIcon(ImportStatus status) {
    IconData icon;
    Color color;

    switch (status) {
      case ImportStatus.idle:
        icon = Icons.hourglass_empty;
        color = AppColors.textMuted;
        break;
      case ImportStatus.importing:
        icon = Icons.sync;
        color = AppColors.primary;
        break;
      case ImportStatus.paused:
        icon = Icons.pause_circle;
        color = AppColors.warning;
        break;
      case ImportStatus.completed:
        icon = Icons.check_circle;
        color = AppColors.success;
        break;
      case ImportStatus.cancelled:
        icon = Icons.cancel;
        color = AppColors.warning;
        break;
      case ImportStatus.failed:
        icon = Icons.error;
        color = AppColors.error;
        break;
    }

    return Icon(icon, color: color, size: 24);
  }

  /// 获取进度条颜色
  Color _getProgressColor(ImportStatus status) {
    switch (status) {
      case ImportStatus.importing:
        return AppColors.primary;
      case ImportStatus.paused:
        return AppColors.warning;
      case ImportStatus.completed:
        return AppColors.success;
      case ImportStatus.cancelled:
      case ImportStatus.failed:
        return AppColors.error;
      case ImportStatus.idle:
        return AppColors.textMuted;
    }
  }

  /// 格式化时间
  String _formatTime(int seconds) {
    if (seconds < 60) {
      return '${seconds}秒';
    } else if (seconds < 3600) {
      return '${(seconds / 60).round()}分钟';
    } else {
      final hours = (seconds / 3600).round();
      final mins = ((seconds % 3600) / 60).round();
      return '${hours}小时${mins}分钟';
    }
  }
}

/// 显示导入进度对话框
///
/// 这是一个便捷函数，用于快速显示模态对话框
Future<void> showImportProgressDialog(
  BuildContext context, {
  bool barrierDismissible = false,
}) {
  return showDialog(
    context: context,
    barrierDismissible: barrierDismissible,
    builder: (context) => const ImportProgressDialog(),
  );
}

/// 导入完成摘要
///
/// 显示导入完成后的摘要信息
class ImportSummaryDialog extends StatelessWidget {
  /// 总文件数
  final int totalFiles;

  /// 成功导入数
  final int successCount;

  /// 失败数
  final int failedCount;

  /// 耗时 (秒)
  final int durationSeconds;

  /// 关闭回调
  final VoidCallback onClose;

  const ImportSummaryDialog({
    super.key,
    required this.totalFiles,
    required this.successCount,
    required this.failedCount,
    required this.durationSeconds,
    required this.onClose,
  });

  @override
  Widget build(BuildContext context) {
    return AlertDialog(
      backgroundColor: AppColors.bgCard,
      title: Row(
        children: [
          Icon(
            failedCount > 0 ? Icons.warning_amber : Icons.check_circle,
            color: failedCount > 0 ? AppColors.warning : AppColors.success,
          ),
          const SizedBox(width: 8),
          const Text('导入完成'),
        ],
      ),
      content: SizedBox(
        width: 300,
        child: Column(
          mainAxisSize: MainAxisSize.min,
          children: [
            // 摘要统计
            _buildSummaryStats(),
            const SizedBox(height: 16),
            // 详细信息
            _buildDetails(),
          ],
        ),
      ),
      actions: [ElevatedButton(onPressed: onClose, child: const Text('确定'))],
    );
  }

  Widget _buildSummaryStats() {
    return Row(
      mainAxisAlignment: MainAxisAlignment.spaceAround,
      children: [
        _buildStatColumn(
          label: '总计',
          value: '$totalFiles',
          color: AppColors.textPrimary,
        ),
        _buildStatColumn(
          label: '成功',
          value: '$successCount',
          color: AppColors.success,
        ),
        _buildStatColumn(
          label: '失败',
          value: '$failedCount',
          color: failedCount > 0 ? AppColors.error : AppColors.textMuted,
        ),
      ],
    );
  }

  Widget _buildStatColumn({
    required String label,
    required String value,
    required Color color,
  }) {
    return Column(
      children: [
        Text(
          value,
          style: TextStyle(
            fontSize: 24,
            fontWeight: FontWeight.bold,
            color: color,
          ),
        ),
        Text(
          label,
          style: const TextStyle(fontSize: 12, color: AppColors.textMuted),
        ),
      ],
    );
  }

  Widget _buildDetails() {
    return Container(
      padding: const EdgeInsets.all(12),
      decoration: BoxDecoration(
        color: AppColors.bgMain,
        borderRadius: BorderRadius.circular(4),
      ),
      child: Row(
        mainAxisAlignment: MainAxisAlignment.center,
        children: [
          const Icon(Icons.timer, size: 16, color: AppColors.textMuted),
          const SizedBox(width: 4),
          Text(
            '耗时: ${_formatDuration(durationSeconds)}',
            style: const TextStyle(
              fontSize: 12,
              color: AppColors.textSecondary,
            ),
          ),
        ],
      ),
    );
  }

  String _formatDuration(int seconds) {
    if (seconds < 60) {
      return '${seconds}秒';
    } else if (seconds < 3600) {
      final mins = seconds ~/ 60;
      final secs = seconds % 60;
      return '$mins分${secs}秒';
    } else {
      final hours = seconds ~/ 3600;
      final mins = (seconds % 3600) ~/ 60;
      return '$hours小时${mins}分钟';
    }
  }
}

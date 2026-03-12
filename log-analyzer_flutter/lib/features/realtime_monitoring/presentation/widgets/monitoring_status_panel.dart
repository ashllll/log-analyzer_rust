import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../../providers/monitoring_provider.dart';

/// 监控状态面板
///
/// 显示实时监控的详细信息
class MonitoringStatusPanel extends ConsumerWidget {
  /// 是否显示面板
  final bool isVisible;

  /// 关闭面板回调
  final VoidCallback? onClose;

  const MonitoringStatusPanel({
    super.key,
    required this.isVisible,
    this.onClose,
  });

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final monitoringState = ref.watch(monitoringProvider);

    if (!isVisible) {
      return const SizedBox.shrink();
    }

    return Card(
      margin: const EdgeInsets.all(8),
      child: Padding(
        padding: const EdgeInsets.all(16),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          mainAxisSize: MainAxisSize.min,
          children: [
            // 标题栏
            Row(
              mainAxisAlignment: MainAxisAlignment.spaceBetween,
              children: [
                const Text(
                  '文件监控状态',
                  style: TextStyle(fontSize: 16, fontWeight: FontWeight.bold),
                ),
                IconButton(
                  icon: const Icon(Icons.close),
                  onPressed: onClose,
                  tooltip: '关闭',
                ),
              ],
            ),
            const Divider(),
            // 监控状态
            _buildStatusRow(
              context,
              '状态',
              monitoringState.isActive ? '运行中' : '已停止',
              monitoringState.isActive ? Colors.green : Colors.red,
            ),
            // 活动指示器
            if (monitoringState.isActive)
              Padding(
                padding: const EdgeInsets.symmetric(vertical: 8),
                child: Row(
                  children: [
                    Container(
                      width: 12,
                      height: 12,
                      decoration: BoxDecoration(
                        shape: BoxShape.circle,
                        color: Colors.green,
                        boxShadow: [
                          BoxShadow(
                            color: Colors.green.withOpacity(0.5),
                            blurRadius: 8,
                            spreadRadius: 2,
                          ),
                        ],
                      ),
                    ),
                    const SizedBox(width: 8),
                    const Text('监控中'),
                  ],
                ),
              ),
            // 已处理事件数
            _buildStatRow(
              context,
              '已处理事件',
              monitoringState.eventsProcessed.toString(),
              Icons.check_circle_outline,
            ),
            // 待处理数
            _buildStatRow(
              context,
              '待处理',
              monitoringState.pendingCount.toString(),
              Icons.pending_actions,
            ),
            // 监控目录数
            _buildStatRow(
              context,
              '监控目录',
              monitoringState.monitoredDirsCount.toString(),
              Icons.folder,
            ),
            // 监控文件数
            _buildStatRow(
              context,
              '监控文件',
              monitoringState.monitoredFilesCount.toString(),
              Icons.insert_drive_file,
            ),
            // 最后更新时间
            if (monitoringState.lastUpdate != null)
              _buildStatRow(
                context,
                '最后更新',
                _formatDateTime(monitoringState.lastUpdate!),
                Icons.access_time,
              ),
            // 错误信息
            if (monitoringState.errorMessage != null) ...[
              const SizedBox(height: 8),
              Container(
                padding: const EdgeInsets.all(8),
                decoration: BoxDecoration(
                  color: Colors.red.withOpacity(0.1),
                  borderRadius: BorderRadius.circular(4),
                  border: Border.all(color: Colors.red.withOpacity(0.3)),
                ),
                child: Row(
                  children: [
                    const Icon(
                      Icons.error_outline,
                      color: Colors.red,
                      size: 16,
                    ),
                    const SizedBox(width: 8),
                    Expanded(
                      child: Text(
                        monitoringState.errorMessage!,
                        style: const TextStyle(color: Colors.red, fontSize: 12),
                      ),
                    ),
                  ],
                ),
              ),
            ],
            // 空占位状态
            if (!monitoringState.isActive &&
                monitoringState.eventsProcessed == 0 &&
                monitoringState.errorMessage == null) ...[
              const SizedBox(height: 16),
              Center(
                child: Text(
                  '点击工具栏按钮启用文件监控',
                  style: TextStyle(
                    color: Colors.grey[600],
                    fontStyle: FontStyle.italic,
                  ),
                ),
              ),
            ],
          ],
        ),
      ),
    );
  }

  /// 构建状态行
  Widget _buildStatusRow(
    BuildContext context,
    String label,
    String value,
    Color valueColor,
  ) {
    return Padding(
      padding: const EdgeInsets.symmetric(vertical: 4),
      child: Row(
        mainAxisAlignment: MainAxisAlignment.spaceBetween,
        children: [
          Text(label, style: const TextStyle(fontWeight: FontWeight.w500)),
          Container(
            padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 4),
            decoration: BoxDecoration(
              color: valueColor.withOpacity(0.1),
              borderRadius: BorderRadius.circular(12),
            ),
            child: Text(
              value,
              style: TextStyle(color: valueColor, fontWeight: FontWeight.bold),
            ),
          ),
        ],
      ),
    );
  }

  /// 构建统计行
  Widget _buildStatRow(
    BuildContext context,
    String label,
    String value,
    IconData icon,
  ) {
    return Padding(
      padding: const EdgeInsets.symmetric(vertical: 4),
      child: Row(
        children: [
          Icon(icon, size: 16, color: Colors.grey[600]),
          const SizedBox(width: 8),
          Text(label),
          const Spacer(),
          Text(value, style: const TextStyle(fontWeight: FontWeight.w500)),
        ],
      ),
    );
  }

  /// 格式化日期时间
  String _formatDateTime(DateTime dateTime) {
    final now = DateTime.now();
    final diff = now.difference(dateTime);

    if (diff.inSeconds < 60) {
      return '${diff.inSeconds}秒前';
    } else if (diff.inMinutes < 60) {
      return '${diff.inMinutes}分钟前';
    } else if (diff.inHours < 24) {
      return '${diff.inHours}小时前';
    } else {
      return '${dateTime.month}/${dateTime.day} ${dateTime.hour}:${dateTime.minute.toString().padLeft(2, '0')}';
    }
  }
}

import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../../providers/monitoring_provider.dart';

/// 监控开关按钮
///
/// 工具栏按钮，用于启用/禁用文件监控
class MonitoringToolbarButton extends ConsumerWidget {
  /// 工作区ID
  final String workspaceId;

  /// 监控路径列表
  final List<String> paths;

  /// 是否显示状态面板
  final bool showStatusPanel;

  /// 状态面板显示回调
  final VoidCallback? onToggleStatusPanel;

  const MonitoringToolbarButton({
    super.key,
    required this.workspaceId,
    required this.paths,
    this.showStatusPanel = false,
    this.onToggleStatusPanel,
  });

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final monitoringState = ref.watch(monitoringProvider);
    final monitoringNotifier = ref.read(monitoringProvider.notifier);

    return Tooltip(
      message: monitoringState.isActive ? '停止监控' : '开始监控',
      child: IconButton(
        icon: Icon(
          monitoringState.isActive ? Icons.visibility : Icons.visibility_off,
          color: monitoringState.isActive ? Colors.green : Colors.red,
        ),
        onPressed: () async {
          if (monitoringState.isActive) {
            await monitoringNotifier.stopMonitoring(workspaceId);
          } else {
            await monitoringNotifier.startMonitoring(workspaceId, paths);
          }
        },
      ),
    );
  }
}

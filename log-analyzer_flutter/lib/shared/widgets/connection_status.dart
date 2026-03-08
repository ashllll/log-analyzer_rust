import 'package:flutter/material.dart';
import '../../core/theme/app_theme.dart';

/// 连接状态指示器
///
/// 对应 React 版本的 ConnectionStatus.tsx
/// 显示应用连接状态
class ConnectionStatus extends StatelessWidget {
  final ConnectionStatusType status;
  final String? message;

  const ConnectionStatus({super.key, required this.status, this.message});

  @override
  Widget build(BuildContext context) {
    final statusInfo = _getStatusInfo();

    return Row(
      mainAxisSize: MainAxisSize.min,
      children: [
        Container(
          width: 8,
          height: 8,
          decoration: BoxDecoration(
            color: statusInfo.color,
            shape: BoxShape.circle,
          ),
        ),
        const SizedBox(width: 6),
        Text(
          statusInfo.label,
          style: const TextStyle(fontSize: 12, color: AppColors.textMuted),
        ),
        if (message != null) ...[
          const SizedBox(width: 8),
          Flexible(
            child: Text(
              message!,
              style: const TextStyle(fontSize: 12, color: AppColors.textMuted),
              overflow: TextOverflow.ellipsis,
            ),
          ),
        ],
      ],
    );
  }

  _StatusInfo _getStatusInfo() {
    switch (status) {
      case ConnectionStatusType.connected:
        return const _StatusInfo(label: '已连接', color: AppColors.success);
      case ConnectionStatusType.disconnected:
        return const _StatusInfo(label: '未连接', color: AppColors.error);
      case ConnectionStatusType.connecting:
        return const _StatusInfo(label: '连接中...', color: AppColors.warning);
      case ConnectionStatusType.reconnecting:
        return const _StatusInfo(label: '重连中...', color: AppColors.warning);
      case ConnectionStatusType.error:
        return const _StatusInfo(label: '连接错误', color: AppColors.error);
    }
  }
}

class _StatusInfo {
  final String label;
  final Color color;

  const _StatusInfo({required this.label, required this.color});
}

/// 连接状态类型
enum ConnectionStatusType {
  connected,
  disconnected,
  connecting,
  reconnecting,
  error,
}

// lib/shared/widgets/empty_state_widget.dart
import 'package:flutter/material.dart';

/// 空状态组件
///
/// 用于显示空列表、无数据等场景
/// 提供统一的视觉提示和操作入口
class EmptyStateWidget extends StatelessWidget {
  /// 图标（必填）
  final IconData icon;

  /// 标题（必填）
  final String title;

  /// 描述文本（可选）
  final String? description;

  /// 操作按钮文本（可选）
  final String? actionLabel;

  /// 操作按钮回调（可选）
  final VoidCallback? onAction;

  const EmptyStateWidget({
    super.key,
    required this.icon,
    required this.title,
    this.description,
    this.actionLabel,
    this.onAction,
  });

  @override
  Widget build(BuildContext context) {
    return Center(
      child: Padding(
        padding: const EdgeInsets.all(32.0),
        child: Column(
          mainAxisAlignment: MainAxisAlignment.center,
          children: [
            // 图标
            Icon(
              icon,
              size: 64,
              color: Colors.grey[500],
            ),
            const SizedBox(height: 16),

            // 标题
            Text(
              title,
              style: Theme.of(context).textTheme.titleMedium?.copyWith(
                    fontSize: 18,
                    fontWeight: FontWeight.w600,
                    color: Colors.grey[300],
                  ),
              textAlign: TextAlign.center,
            ),

            // 描述
            if (description != null) ...[
              const SizedBox(height: 8),
              Text(
                description!,
                style: Theme.of(context).textTheme.bodyMedium?.copyWith(
                      fontSize: 14,
                      color: Colors.grey[500],
                    ),
                textAlign: TextAlign.center,
              ),
            ],

            // 操作按钮
            if (actionLabel != null && onAction != null) ...[
              const SizedBox(height: 24),
              ElevatedButton.icon(
                onPressed: onAction,
                icon: const Icon(Icons.add),
                label: Text(actionLabel!),
              ),
            ],
          ],
        ),
      ),
    );
  }
}

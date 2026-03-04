import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../../../../shared/providers/app_provider.dart';
import '../../providers/settings_provider.dart';

/// 搜索设置 Tab
///
/// 包含搜索历史记录数设置
class SearchSettingsTab extends ConsumerWidget {
  const SearchSettingsTab({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final settingsState = ref.watch(settingsProvider);

    return SingleChildScrollView(
      padding: const EdgeInsets.all(24),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          const Text(
            '搜索设置',
            style: TextStyle(
              fontSize: 24,
              fontWeight: FontWeight.bold,
            ),
          ),
          const SizedBox(height: 32),

          // 搜索历史限制
          Card(
            child: Padding(
              padding: const EdgeInsets.all(20),
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  Row(
                    children: [
                      Icon(
                        Icons.history,
                        color: Theme.of(context).colorScheme.primary,
                      ),
                      const SizedBox(width: 12),
                      const Text(
                        '搜索历史记录数',
                        style: TextStyle(
                          fontSize: 18,
                          fontWeight: FontWeight.w600,
                        ),
                      ),
                    ],
                  ),
                  const SizedBox(height: 16),
                  const Text(
                    '设置搜索历史记录的最大保存数量',
                    style: TextStyle(
                      color: Colors.grey,
                    ),
                  ),
                  const SizedBox(height: 24),

                  // 滑块
                  Row(
                    children: [
                      const Text('10'),
                      Expanded(
                        child: Slider(
                          value: settingsState.searchHistoryLimit.toDouble(),
                          min: 10,
                          max: 200,
                          divisions: 19,
                          label: '${settingsState.searchHistoryLimit}',
                          onChanged: (value) {
                            ref.read(settingsProvider.notifier)
                                .setSearchHistoryLimit(value.round());
                          },
                          onChangeEnd: (value) {
                            ref.read(appStateProvider.notifier).addToast(
                              ToastType.success,
                              '搜索历史限制已设置为 ${value.round()} 条',
                            );
                          },
                        ),
                      ),
                      const Text('200'),
                    ],
                  ),

                  const SizedBox(height: 8),

                  // 当前值显示
                  Center(
                    child: Container(
                      padding: const EdgeInsets.symmetric(
                        horizontal: 16,
                        vertical: 8,
                      ),
                      decoration: BoxDecoration(
                        color: Theme.of(context).colorScheme.primaryContainer,
                        borderRadius: BorderRadius.circular(20),
                      ),
                      child: Text(
                        '当前: ${settingsState.searchHistoryLimit} 条',
                        style: TextStyle(
                          color: Theme.of(context).colorScheme.onPrimaryContainer,
                          fontWeight: FontWeight.w600,
                        ),
                      ),
                    ),
                  ),
                ],
              ),
            ),
          ),

          const SizedBox(height: 16),

          // 说明
          Card(
            child: Padding(
              padding: const EdgeInsets.all(20),
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  Row(
                    children: [
                      Icon(
                        Icons.info_outline,
                        color: Theme.of(context).colorScheme.primary,
                      ),
                      const SizedBox(width: 12),
                      const Text(
                        '使用说明',
                        style: TextStyle(
                          fontSize: 18,
                          fontWeight: FontWeight.w600,
                        ),
                      ),
                    ],
                  ),
                  const SizedBox(height: 16),
                  _buildInfoItem(
                    Icons.search,
                    '搜索历史',
                    '在搜索框中点击可显示历史搜索记录',
                  ),
                  const SizedBox(height: 12),
                  _buildInfoItem(
                    Icons.storage,
                    '存储建议',
                    '数值越大占用的存储空间越多，建议 50-100 条',
                  ),
                  const SizedBox(height: 12),
                  _buildInfoItem(
                    Icons.delete_sweep,
                    '自动清理',
                    '超出限制时会自动删除最早的记录',
                  ),
                ],
              ),
            ),
          ),
        ],
      ),
    );
  }

  Widget _buildInfoItem(IconData icon, String title, String description) {
    return Row(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        Icon(icon, size: 20, color: Colors.grey),
        const SizedBox(width: 12),
        Expanded(
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              Text(
                title,
                style: const TextStyle(
                  fontWeight: FontWeight.w500,
                ),
              ),
              const SizedBox(height: 2),
              Text(
                description,
                style: TextStyle(
                  fontSize: 13,
                  color: Colors.grey[600],
                ),
              ),
            ],
          ),
        ),
      ],
    );
  }
}

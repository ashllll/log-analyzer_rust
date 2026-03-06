import 'package:flutter/material.dart';
import 'package:shimmer/shimmer.dart';

/// 文件树骨架屏加载动画
///
/// 模拟文件树节点行（图标 + 文件名）的加载状态
class FileTreeLoadingSkeleton extends StatelessWidget {
  /// 行数
  final int itemCount;

  const FileTreeLoadingSkeleton({
    super.key,
    this.itemCount = 10,
  });

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final isDark = theme.brightness == Brightness.dark;

    return Shimmer.fromColors(
      baseColor: isDark ? Colors.grey[800]! : Colors.grey[300]!,
      highlightColor: isDark ? Colors.grey[700]! : Colors.grey[100]!,
      child: ListView.builder(
        padding: const EdgeInsets.symmetric(vertical: 8),
        itemCount: itemCount,
        itemBuilder: (context, index) => Padding(
          padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 8),
          child: Row(
            children: [
              // 文件图标
              Container(
                width: 16,
                height: 16,
                decoration: BoxDecoration(
                  color: Colors.white,
                  borderRadius: BorderRadius.circular(4),
                ),
              ),
              const SizedBox(width: 8),
              // 文件名
              Container(
                width: 120 + (index % 3) * 30.0,
                height: 14,
                decoration: BoxDecoration(
                  color: Colors.white,
                  borderRadius: BorderRadius.circular(4),
                ),
              ),
            ],
          ),
        ),
      ),
    );
  }
}

/// 文件预览骨架屏加载动画
///
/// 模拟文件内容行的加载状态
class FilePreviewLoadingSkeleton extends StatelessWidget {
  /// 行数
  final int lineCount;

  const FilePreviewLoadingSkeleton({
    super.key,
    this.lineCount = 15,
  });

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final isDark = theme.brightness == Brightness.dark;

    return Shimmer.fromColors(
      baseColor: isDark ? Colors.grey[800]! : Colors.grey[300]!,
      highlightColor: isDark ? Colors.grey[700]! : Colors.grey[100]!,
      child: Padding(
        padding: const EdgeInsets.all(16),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: List.generate(lineCount, (index) {
            // 最后一行较短，其他行随机长度
            final isLast = index == lineCount - 1;
            final width = isLast
                ? 80.0
                : 200.0 + (index % 4) * 50.0;

            return Padding(
              padding: const EdgeInsets.only(bottom: 8),
              child: Container(
                width: width,
                height: 14,
                decoration: BoxDecoration(
                  color: Colors.white,
                  borderRadius: BorderRadius.circular(4),
                ),
              ),
            );
          }),
        ),
      ),
    );
  }
}

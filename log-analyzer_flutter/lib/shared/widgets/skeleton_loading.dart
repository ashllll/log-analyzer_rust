// lib/shared/widgets/skeleton_loading.dart
import 'package:flutter/material.dart';
import 'package:shimmer/shimmer.dart';

/// 骨架屏加载组件
///
/// 使用 shimmer 效果实现骨架屏动画
/// 用于在数据加载时显示占位内容
class SkeletonLoading extends StatelessWidget {
  /// 宽度（可选，默认 full）
  final double? width;

  /// 高度（可选，默认 16）
  final double? height;

  /// 边框半径
  final double borderRadius;

  /// 是否启用动画
  final bool enabled;

  /// 自定义基础颜色
  final Color? baseColor;

  /// 自定义高亮颜色
  final Color? highlightColor;

  const SkeletonLoading({
    super.key,
    this.width,
    this.height,
    this.borderRadius = 4,
    this.enabled = true,
    this.baseColor,
    this.highlightColor,
  });

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final isDark = theme.brightness == Brightness.dark;

    return Shimmer.fromColors(
      enabled: enabled,
      baseColor: baseColor ?? (isDark ? Colors.grey[800]! : Colors.grey[300]!),
      highlightColor: highlightColor ?? (isDark ? Colors.grey[700]! : Colors.grey[100]!),
      child: Container(
        width: width,
        height: height ?? 16,
        decoration: BoxDecoration(
          color: Colors.white,
          borderRadius: BorderRadius.circular(borderRadius),
        ),
      ),
    );
  }
}

/// 骨架屏列表项
///
/// 用于列表项的占位显示
class SkeletonListItem extends StatelessWidget {
  /// 高度
  final double height;

  /// 左边图标占位宽度
  final double iconWidth;

  /// 是否有副标题
  final bool hasSubtitle;

  const SkeletonListItem({
    super.key,
    this.height = 56,
    this.iconWidth = 40,
    this.hasSubtitle = false,
  });

  @override
  Widget build(BuildContext context) {
    return Container(
      padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 8),
      height: height,
      child: Row(
        children: [
          // 左侧图标占位
          const SkeletonLoading(
            width: 40,
            height: 40,
            borderRadius: 8,
          ),
          const SizedBox(width: 12),
          // 右侧内容
          Expanded(
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              mainAxisAlignment: MainAxisAlignment.center,
              children: [
                SkeletonLoading(
                  height: 14,
                  borderRadius: 2,
                ),
                if (hasSubtitle) ...[
                  const SizedBox(height: 8),
                  SkeletonLoading(
                    width: 120,
                    height: 12,
                    borderRadius: 2,
                  ),
                ],
              ],
            ),
          ),
        ],
      ),
    );
  }
}

/// 骨架屏卡片
///
/// 用于卡片内容的占位显示
class SkeletonCard extends StatelessWidget {
  /// 高度
  final double height;

  /// 是否有副标题
  final bool hasSubtitle;

  /// 是否有底部操作栏
  final bool hasActions;

  const SkeletonCard({
    super.key,
    this.height = 120,
    this.hasSubtitle = true,
    this.hasActions = true,
  });

  @override
  Widget build(BuildContext context) {
    return Container(
      padding: const EdgeInsets.all(16),
      height: height,
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          // 标题行
          Row(
            children: [
              const SkeletonLoading(
                width: 16,
                height: 16,
                borderRadius: 4,
              ),
              const SizedBox(width: 8),
              Expanded(
                child: SkeletonLoading(
                  height: 18,
                  borderRadius: 4,
                ),
              ),
            ],
          ),
          const SizedBox(height: 12),
          // 副标题
          if (hasSubtitle)
            const SkeletonLoading(
              width: 200,
              height: 14,
              borderRadius: 4,
            ),
          if (hasSubtitle) const SizedBox(height: 8),
          // 描述行
          const SkeletonLoading(
            height: 12,
            borderRadius: 4,
          ),
          const Spacer(),
          // 底部操作栏
          if (hasActions)
            Row(
              mainAxisAlignment: MainAxisAlignment.end,
              children: [
                const SkeletonLoading(
                  width: 60,
                  height: 32,
                  borderRadius: 4,
                ),
                const SizedBox(width: 8),
                const SkeletonLoading(
                  width: 60,
                  height: 32,
                  borderRadius: 4,
                ),
              ],
            ),
        ],
      ),
    );
  }
}

/// 骨架屏列表
///
/// 用于生成多个骨架屏列表项
class SkeletonList extends StatelessWidget {
  /// 项目数量
  final int itemCount;

  /// 列表项高度
  final double itemHeight;

  /// 是否有副标题
  final bool hasSubtitle;

  const SkeletonList({
    super.key,
    this.itemCount = 5,
    this.itemHeight = 56,
    this.hasSubtitle = false,
  });

  @override
  Widget build(BuildContext context) {
    return ListView.builder(
      itemCount: itemCount,
      itemBuilder: (context, index) => SkeletonListItem(
        height: itemHeight,
        hasSubtitle: hasSubtitle,
      ),
    );
  }
}

/// 骨架屏网格
///
/// 用于生成网格状的骨架屏
class SkeletonGrid extends StatelessWidget {
  /// 列数
  final int crossAxisCount;

  /// 项目数量
  final int itemCount;

  /// 网格项高度
  final double itemHeight;

  /// 交叉轴间距
  final double crossAxisSpacing;

  /// 主轴间距
  final double mainAxisSpacing;

  const SkeletonGrid({
    super.key,
    this.crossAxisCount = 2,
    this.itemCount = 6,
    this.itemHeight = 120,
    this.crossAxisSpacing = 16,
    this.mainAxisSpacing = 16,
  });

  @override
  Widget build(BuildContext context) {
    return GridView.builder(
      padding: const EdgeInsets.all(16),
      gridDelegate: SliverGridDelegateWithFixedCrossAxisCount(
        crossAxisCount: crossAxisCount,
        mainAxisSpacing: mainAxisSpacing,
        crossAxisSpacing: crossAxisSpacing,
        childAspectRatio: 1.5,
        mainAxisExtent: itemHeight,
      ),
      itemCount: itemCount,
      itemBuilder: (context, index) => SkeletonCard(
        height: itemHeight,
      ),
    );
  }
}

/// 搜索结果骨架屏
///
/// 用于搜索结果列表加载时显示
class SearchResultSkeleton extends StatelessWidget {
  /// 显示的项目数量
  final int itemCount;

  const SearchResultSkeleton({
    super.key,
    this.itemCount = 10,
  });

  @override
  Widget build(BuildContext context) {
    return ListView.builder(
      itemCount: itemCount,
      padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 8),
      itemBuilder: (context, index) => Container(
        padding: const EdgeInsets.symmetric(vertical: 8),
        child: Row(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            // 行号占位
            const SkeletonLoading(
              width: 50,
              height: 16,
            ),
            const SizedBox(width: 12),
            // 时间戳占位
            const SkeletonLoading(
              width: 80,
              height: 16,
            ),
            const SizedBox(width: 12),
            // 日志内容占位
            Expanded(
              child: SkeletonLoading(
                height: 16,
              ),
            ),
          ],
        ),
      ),
    );
  }
}

/// 工作区列表骨架屏
///
/// 用于工作区列表加载时显示
class WorkspaceListSkeleton extends StatelessWidget {
  /// 显示的项目数量
  final int itemCount;

  const WorkspaceListSkeleton({
    super.key,
    this.itemCount = 3,
  });

  @override
  Widget build(BuildContext context) {
    return ListView.builder(
      padding: const EdgeInsets.all(16),
      itemCount: itemCount,
      itemBuilder: (context, index) => const Padding(
        padding: EdgeInsets.only(bottom: 12),
        child: SkeletonCard(
          height: 140,
          hasSubtitle: true,
          hasActions: true,
        ),
      ),
    );
  }
}

import 'package:flutter/material.dart';

import '../../core/constants/app_constants.dart';

/// 虚拟滚动日志列表组件
///
/// 使用 Flutter 内置的 ListView.builder 实现高效渲染
class VirtualLogList extends StatefulWidget {
  /// 日志条目总数
  final int itemCount;

  /// 项目构建器
  final Widget Function(BuildContext context, int index) itemBuilder;

  /// 项目高度（用于计算可见区域）
  final double itemHeight;

  /// 是否启用动态高度（默认 false）
  final bool dynamicHeight;

  /// 滚动控制器
  final ScrollController? scrollController;

  /// 列表内边距
  final EdgeInsets? padding;

  /// 滚动到索引的回调
  final ValueChanged<int>? onIndexChanged;

  /// 是否显示分割线
  final bool showDividers;

  /// 缓存区域大小（渲染区域之外的预渲染区域）
  ///
  /// 值越大，滚动时加载越平滑，但内存占用越高
  /// 默认值为 itemHeight * 10，兼顾性能和内存
  final double cacheExtent;

  /// 是否启用粘性头部
  final bool shrinkWrap;

  const VirtualLogList({
    super.key,
    required this.itemCount,
    required this.itemBuilder,
    this.itemHeight = AppConstants.logItemHeight,
    this.dynamicHeight = false,
    this.scrollController,
    this.padding,
    this.onIndexChanged,
    this.showDividers = true,
    this.cacheExtent = 0, // 默认使用 itemHeight * 10
    this.shrinkWrap = false,
  });

  @override
  State<VirtualLogList> createState() => _VirtualLogListState();
}

class _VirtualLogListState extends State<VirtualLogList> {
  ScrollController? _scrollController;
  int? _lastVisibleIndex;

  @override
  void initState() {
    super.initState();
    _scrollController = widget.scrollController ?? ScrollController();
    _setupScrollListener();
  }

  @override
  void didUpdateWidget(VirtualLogList oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (widget.scrollController != oldWidget.scrollController) {
      if (widget.scrollController == null && _scrollController != null) {
        _scrollController!.dispose();
      }
      _scrollController = widget.scrollController ?? ScrollController();
      _setupScrollListener();
    }
  }

  @override
  void dispose() {
    _scrollController?.removeListener(_onScrollChanged);
    if (widget.scrollController == null) {
      _scrollController?.dispose();
    }
    super.dispose();
  }

  void _setupScrollListener() {
    _scrollController?.addListener(_onScrollChanged);
  }

  void _onScrollChanged() {
    if (!widget.dynamicHeight && widget.itemHeight > 0) {
      final index = (_scrollController!.offset / widget.itemHeight).floor();
      if (index != _lastVisibleIndex && index >= 0 && index < widget.itemCount) {
        _lastVisibleIndex = index;
        widget.onIndexChanged?.call(index);
      }
    }
  }

  @override
  Widget build(BuildContext context) {
    // 计算缓存区域大小：如果未指定，则使用 itemHeight * 10 作为默认值
    final effectiveCacheExtent = widget.cacheExtent > 0
        ? widget.cacheExtent
        : widget.itemHeight * 10;

    return ListView.builder(
      controller: _scrollController,
      padding: widget.padding,
      itemCount: widget.itemCount,
      // 虚拟滚动优化：设置缓存区域大小
      cacheExtent: effectiveCacheExtent,
      // 虚拟滚动优化：对于小列表可启用 shrinkWrap
      shrinkWrap: widget.shrinkWrap,
      // 使用 itemExtent 固定高度可大幅提升性能
      itemExtent: widget.dynamicHeight ? null : widget.itemHeight,
      // 如果是动态高度，可以使用 prototypeItem 优化
      prototypeItem: widget.dynamicHeight
          ? null
          : widget.itemBuilder(context, 0),
      itemBuilder: (context, index) {
        final item = widget.itemBuilder(context, index);
        if (!widget.showDividers || index == widget.itemCount - 1) {
          return item;
        }
        return Column(
          children: [
            item,
            const Divider(height: 1),
          ],
        );
      },
    );
  }
}

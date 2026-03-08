import 'package:flutter/material.dart';

import '../../../../core/theme/app_theme.dart';

/// 搜索进度条组件
///
/// 显示搜索任务进度，包括：
/// - 线性进度条 (0-100%)
/// - 已扫描文件数
/// - 已找到结果数
/// - 取消按钮
/// - 搜索完成后显示"完成"状态，几秒后自动消失
class SearchProgressBar extends StatefulWidget {
  /// 进度百分比 (0-100)
  final int progress;

  /// 已扫描文件数
  final int scannedFiles;

  /// 已找到结果数
  final int resultsFound;

  /// 取消搜索回调
  final VoidCallback? onCancel;

  /// 是否搜索完成
  final bool isCompleted;

  const SearchProgressBar({
    super.key,
    required this.progress,
    required this.scannedFiles,
    required this.resultsFound,
    this.onCancel,
    this.isCompleted = false,
  });

  @override
  State<SearchProgressBar> createState() => _SearchProgressBarState();
}

class _SearchProgressBarState extends State<SearchProgressBar>
    with SingleTickerProviderStateMixin {
  late AnimationController _fadeController;
  late Animation<double> _fadeAnimation;
  bool _showCompleted = false;

  @override
  void initState() {
    super.initState();
    _fadeController = AnimationController(
      duration: const Duration(milliseconds: 300),
      vsync: this,
    );
    _fadeAnimation = Tween<double>(
      begin: 1.0,
      end: 0.0,
    ).animate(CurvedAnimation(parent: _fadeController, curve: Curves.easeOut));
  }

  @override
  void didUpdateWidget(SearchProgressBar oldWidget) {
    super.didUpdateWidget(oldWidget);
    // 当搜索完成时，显示"完成"状态，几秒后淡出
    if (widget.isCompleted && !oldWidget.isCompleted) {
      setState(() {
        _showCompleted = true;
      });
      Future.delayed(const Duration(seconds: 2), () {
        if (mounted) {
          _fadeController.forward().then((_) {
            if (mounted) {
              setState(() {
                _showCompleted = false;
              });
            }
          });
        }
      });
    }
  }

  @override
  void dispose() {
    _fadeController.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    if (!_showCompleted && widget.progress == 0 && widget.scannedFiles == 0) {
      return const SizedBox.shrink();
    }

    return FadeTransition(
      opacity: _fadeAnimation,
      child: Container(
        padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 12),
        decoration: const BoxDecoration(
          color: AppColors.bgCard,
          border: Border(bottom: BorderSide(color: AppColors.border, width: 1)),
        ),
        child: Row(
          children: [
            // 进度信息
            Expanded(
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                mainAxisSize: MainAxisSize.min,
                children: [
                  // 状态文本
                  Text(
                    _showCompleted
                        ? '搜索完成'
                        : (widget.isCompleted ? '完成' : '搜索中...'),
                    style: TextStyle(
                      fontSize: 12,
                      fontWeight: FontWeight.w500,
                      color: _showCompleted
                          ? AppColors.success
                          : AppColors.textSecondary,
                    ),
                  ),
                  const SizedBox(height: 6),
                  // 进度条
                  ClipRRect(
                    borderRadius: BorderRadius.circular(2),
                    child: LinearProgressIndicator(
                      value: widget.isCompleted
                          ? 1.0
                          : (widget.progress / 100).clamp(0.0, 1.0),
                      backgroundColor: AppColors.bgMain,
                      valueColor: AlwaysStoppedAnimation<Color>(
                        _showCompleted
                            ? AppColors.success
                            : (widget.isCompleted
                                  ? AppColors.success
                                  : AppColors.primary),
                      ),
                      minHeight: 4,
                    ),
                  ),
                  const SizedBox(height: 6),
                  // 统计信息
                  Text(
                    _showCompleted
                        ? '已找到 ${widget.resultsFound} 条结果'
                        : '已扫描 ${widget.scannedFiles} 文件，已找到 ${widget.resultsFound} 条结果',
                    style: const TextStyle(
                      fontSize: 11,
                      color: AppColors.textMuted,
                    ),
                  ),
                ],
              ),
            ),
            // 取消按钮
            if (widget.onCancel != null &&
                !widget.isCompleted &&
                !_showCompleted)
              Padding(
                padding: const EdgeInsets.only(left: 12),
                child: IconButton(
                  icon: const Icon(
                    Icons.close,
                    size: 18,
                    color: AppColors.textMuted,
                  ),
                  onPressed: widget.onCancel,
                  tooltip: '取消搜索',
                  style: IconButton.styleFrom(
                    backgroundColor: AppColors.bgHover,
                    padding: const EdgeInsets.all(8),
                  ),
                ),
              ),
          ],
        ),
      ),
    );
  }
}

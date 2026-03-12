import 'package:flutter/material.dart';
import 'package:flutter/services.dart';

import '../../../../shared/models/common.dart';
import '../../../../core/theme/app_theme.dart';

/// 日志详情面板组件
///
/// 功能：
/// - 全屏详情视图（Dialog 宽度 90% 视口，高度 90% 视口）
/// - 显示文件名、时间戳、日志级别
/// - 显示完整日志行内容（monospace 字体）
/// - 显示上下文（前后的日志行）
/// - Esc 键关闭详情面板
/// - 点击列表行切换详情显示
///
/// 对应 CONTEXT.md 中的"日志详情面板"设计要求
class LogDetailPanel extends StatefulWidget {
  /// 日志条目
  final LogEntry entry;

  /// 关闭回调
  final VoidCallback onClose;

  /// 可选：上下文行列表（用于显示前后日志）
  final List<LogEntry>? contextLines;

  /// 搜索关键词（用于高亮显示）
  final List<String>? keywords;

  const LogDetailPanel({
    super.key,
    required this.entry,
    required this.onClose,
    this.contextLines,
    this.keywords,
  });

  @override
  State<LogDetailPanel> createState() => _LogDetailPanelState();
}

class _LogDetailPanelState extends State<LogDetailPanel> {
  /// 键盘焦点节点
  final FocusNode _focusNode = FocusNode();

  /// 滚动控制器
  final ScrollController _scrollController = ScrollController();

  /// 当前滚动位置（用于追踪匹配行）
  int _currentMatchIndex = 0;

  /// 上下文行列表
  List<LogEntry> _displayLines = [];

  @override
  void initState() {
    super.initState();
    _loadContextLines();
    // 请求焦点以接收键盘事件
    WidgetsBinding.instance.addPostFrameCallback((_) {
      _focusNode.requestFocus();
    });
  }

  @override
  void dispose() {
    _focusNode.dispose();
    _scrollController.dispose();
    super.dispose();
  }

  /// 加载上下文行
  ///
  /// 初始化时加载前后 100 行上下文（滑动窗口）
  void _loadContextLines() {
    // 如果有预提供的上下文行，直接使用
    if (widget.contextLines != null && widget.contextLines!.isNotEmpty) {
      _displayLines = widget.contextLines!;
    } else {
      // 否则只显示当前日志条目
      _displayLines = [widget.entry];
    }

    // 找到当前匹配行在列表中的位置
    _currentMatchIndex = _displayLines.indexWhere(
      (line) => line.id == widget.entry.id,
    );
    if (_currentMatchIndex == -1) {
      _currentMatchIndex = 0;
    }
  }

  /// 处理键盘事件
  void _handleKeyEvent(KeyEvent event) {
    if (event is KeyDownEvent) {
      // Esc 键关闭
      if (event.logicalKey == LogicalKeyboardKey.escape) {
        widget.onClose();
      }
      // 上箭头 - 跳到上一个匹配
      else if (event.logicalKey == LogicalKeyboardKey.arrowUp) {
        _jumpToPreviousMatch();
      }
      // 下箭头 - 跳到下一个匹配
      else if (event.logicalKey == LogicalKeyboardKey.arrowDown) {
        _jumpToNextMatch();
      }
    }
  }

  /// 跳到上一个匹配
  void _jumpToPreviousMatch() {
    if (_displayLines.isEmpty) return;

    // 找到所有匹配行
    final matchIndices = <int>[];
    for (int i = 0; i < _displayLines.length; i++) {
      if (_displayLines[i].matchedKeywords != null &&
          _displayLines[i].matchedKeywords!.isNotEmpty) {
        matchIndices.add(i);
      }
    }

    if (matchIndices.isEmpty) return;

    // 找到上一个匹配
    final currentMatchInList = matchIndices.indexWhere(
      (idx) => idx >= _currentMatchIndex,
    );
    if (currentMatchInList > 0) {
      _currentMatchIndex = matchIndices[currentMatchInList - 1];
      _scrollToIndex(_currentMatchIndex);
    }
  }

  /// 跳到下一个匹配
  void _jumpToNextMatch() {
    if (_displayLines.isEmpty) return;

    // 找到所有匹配行
    final matchIndices = <int>[];
    for (int i = 0; i < _displayLines.length; i++) {
      if (_displayLines[i].matchedKeywords != null &&
          _displayLines[i].matchedKeywords!.isNotEmpty) {
        matchIndices.add(i);
      }
    }

    if (matchIndices.isEmpty) return;

    // 找到下一个匹配
    for (int i = 0; i < matchIndices.length; i++) {
      if (matchIndices[i] > _currentMatchIndex) {
        _currentMatchIndex = matchIndices[i];
        _scrollToIndex(_currentMatchIndex);
        return;
      }
    }
  }

  /// 滚动到指定索引
  void _scrollToIndex(int index) {
    const lineHeight = 28.0; // 每行的高度
    final targetOffset = index * lineHeight;

    _scrollController.animateTo(
      targetOffset,
      duration: const Duration(milliseconds: 200),
      curve: Curves.easeInOut,
    );
  }

  @override
  Widget build(BuildContext context) {
    final screenSize = MediaQuery.of(context).size;

    return Dialog(
      backgroundColor: AppColors.bgMain,
      insetPadding: EdgeInsets.symmetric(
        horizontal: screenSize.width * 0.05,
        vertical: screenSize.height * 0.05,
      ),
      child: KeyboardListener(
        focusNode: _focusNode,
        onKeyEvent: _handleKeyEvent,
        child: Container(
          width: screenSize.width * 0.9,
          height: screenSize.height * 0.9,
          decoration: BoxDecoration(
            color: AppColors.bgMain,
            borderRadius: BorderRadius.circular(12),
            border: Border.all(color: AppColors.border, width: 1),
          ),
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.stretch,
            children: [
              // 标题栏
              _buildHeader(screenSize),
              const Divider(height: 1, color: AppColors.border),
              // 快捷键提示
              _buildKeyboardHints(),
              const Divider(height: 1, color: AppColors.border),
              // 日志内容
              Expanded(child: _buildLogContent()),
            ],
          ),
        ),
      ),
    );
  }

  /// 构建标题栏
  Widget _buildHeader(Size screenSize) {
    return Container(
      padding: const EdgeInsets.symmetric(horizontal: 20, vertical: 16),
      decoration: const BoxDecoration(
        color: AppColors.bgCard,
        borderRadius: BorderRadius.vertical(top: Radius.circular(12)),
      ),
      child: Row(
        children: [
          // 文件名和行号
          Expanded(
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Text(
                  widget.entry.file,
                  style: const TextStyle(
                    fontSize: 16,
                    fontWeight: FontWeight.bold,
                    color: AppColors.textPrimary,
                  ),
                ),
                const SizedBox(height: 4),
                Row(
                  children: [
                    // 时间戳
                    Text(
                      widget.entry.timestamp,
                      style: const TextStyle(
                        fontSize: 13,
                        fontFamily: 'FiraCode',
                        color: AppColors.textMuted,
                      ),
                    ),
                    const SizedBox(width: 16),
                    // 行号
                    Text(
                      '行 ${widget.entry.line}',
                      style: const TextStyle(
                        fontSize: 13,
                        color: AppColors.textMuted,
                      ),
                    ),
                  ],
                ),
              ],
            ),
          ),
          // 日志级别标签
          _buildLevelChip(widget.entry.level),
          const SizedBox(width: 16),
          // 关闭按钮
          IconButton(
            icon: const Icon(Icons.close),
            onPressed: widget.onClose,
            tooltip: '关闭 (Esc)',
            color: AppColors.textMuted,
          ),
        ],
      ),
    );
  }

  /// 构建快捷键提示
  Widget _buildKeyboardHints() {
    return Container(
      padding: const EdgeInsets.symmetric(horizontal: 20, vertical: 8),
      color: AppColors.bgCard,
      child: Row(
        mainAxisAlignment: MainAxisAlignment.center,
        children: [
          _buildHintChip('Esc', '关闭'),
          const SizedBox(width: 16),
          _buildHintChip('\u2191', '上一条'),
          const SizedBox(width: 16),
          _buildHintChip('\u2193', '下一条'),
        ],
      ),
    );
  }

  /// 构建快捷键提示标签
  Widget _buildHintChip(String key, String action) {
    return Row(
      mainAxisSize: MainAxisSize.min,
      children: [
        Container(
          padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 2),
          decoration: BoxDecoration(
            color: AppColors.bgInput,
            borderRadius: BorderRadius.circular(4),
            border: Border.all(color: AppColors.border, width: 1),
          ),
          child: Text(
            key,
            style: const TextStyle(
              fontSize: 12,
              fontFamily: 'FiraCode',
              color: AppColors.textSecondary,
            ),
          ),
        ),
        const SizedBox(width: 4),
        Text(
          action,
          style: const TextStyle(fontSize: 12, color: AppColors.textMuted),
        ),
      ],
    );
  }

  /// 构建日志级别标签
  Widget _buildLevelChip(String level) {
    Color color;
    switch (level.toUpperCase()) {
      case 'ERROR':
      case 'FATAL':
        color = AppColors.error;
        break;
      case 'WARN':
      case 'WARNING':
        color = AppColors.warning;
        break;
      case 'INFO':
        color = AppColors.primary;
        break;
      case 'DEBUG':
        color = AppColors.keywordPurple;
        break;
      case 'TRACE':
        color = AppColors.textMuted;
        break;
      default:
        color = AppColors.textMuted;
    }

    return Container(
      padding: const EdgeInsets.symmetric(horizontal: 10, vertical: 4),
      decoration: BoxDecoration(
        color: color.withValues(alpha: 0.15),
        borderRadius: BorderRadius.circular(6),
        border: Border.all(color: color.withValues(alpha: 0.3), width: 1),
      ),
      child: Text(
        level.toUpperCase(),
        style: TextStyle(
          color: color,
          fontSize: 13,
          fontWeight: FontWeight.w600,
          letterSpacing: 0.5,
        ),
      ),
    );
  }

  /// 构建日志内容
  Widget _buildLogContent() {
    return ListView.builder(
      controller: _scrollController,
      padding: const EdgeInsets.all(16),
      itemCount: _displayLines.length,
      itemExtent: 28.0, // 固定行高以提高性能
      itemBuilder: (context, index) {
        final line = _displayLines[index];
        final isMatch =
            line.id == widget.entry.id ||
            (line.matchedKeywords != null && line.matchedKeywords!.isNotEmpty);

        return _buildLogLine(line, isMatch, index);
      },
    );
  }

  /// 构建单条日志行
  Widget _buildLogLine(LogEntry line, bool isMatch, int index) {
    // 确定行号宽度（根据最大行数计算）
    final maxLineNum = _displayLines.isNotEmpty
        ? _displayLines.map((e) => e.line).reduce((a, b) => a > b ? a : b)
        : 99999;
    final lineNumWidth = maxLineNum.toString().length * 10.0 + 20;

    return Container(
      height: 28,
      padding: const EdgeInsets.symmetric(horizontal: 8),
      decoration: BoxDecoration(
        color: isMatch ? AppColors.bgHover : Colors.transparent,
        border: isMatch
            ? Border.all(
                color: AppColors.primary.withValues(alpha: 0.3),
                width: 1,
              )
            : null,
        borderRadius: BorderRadius.circular(4),
      ),
      child: Row(
        crossAxisAlignment: CrossAxisAlignment.center,
        children: [
          // 行号
          SizedBox(
            width: lineNumWidth,
            child: Text(
              line.line.toString(),
              style: const TextStyle(
                fontFamily: 'FiraCode',
                fontSize: 13,
                color: AppColors.textMuted,
              ),
            ),
          ),
          const SizedBox(width: 12),
          // 日志内容（带关键词高亮）
          Expanded(child: _buildHighlightedContent(line)),
        ],
      ),
    );
  }

  /// 构建带高亮的日志内容
  Widget _buildHighlightedContent(LogEntry line) {
    final keywords = widget.keywords ?? line.matchedKeywords ?? [];

    if (keywords.isEmpty) {
      return Text(
        line.content,
        style: const TextStyle(
          fontFamily: 'FiraCode',
          fontSize: 13,
          color: AppColors.textSecondary,
        ),
        maxLines: 1,
        overflow: TextOverflow.ellipsis,
      );
    }

    // 构建高亮片段
    final spans = _buildHighlightedSpans(line.content, keywords);

    return Text.rich(
      TextSpan(
        children: spans,
        style: const TextStyle(fontFamily: 'FiraCode', fontSize: 13),
      ),
      maxLines: 1,
      overflow: TextOverflow.ellipsis,
    );
  }

  /// 构建高亮文本片段
  List<TextSpan> _buildHighlightedSpans(String content, List<String> keywords) {
    final spans = <TextSpan>[];
    var currentIndex = 0;

    // 收集所有匹配位置
    final matches = <_MatchPosition>[];
    for (final keyword in keywords) {
      var index = content.indexOf(keyword, 0);
      while (index != -1) {
        matches.add(
          _MatchPosition(
            keyword: keyword,
            index: index,
            length: keyword.length,
          ),
        );
        index = content.indexOf(keyword, index + keyword.length);
      }
    }

    // 按位置排序
    matches.sort((a, b) => a.index.compareTo(b.index));

    // 构建文本片段
    for (final match in matches) {
      // 添加普通文本
      if (match.index > currentIndex) {
        spans.add(
          TextSpan(
            text: content.substring(currentIndex, match.index),
            style: const TextStyle(
              fontFamily: 'FiraCode',
              fontSize: 13,
              color: AppColors.textSecondary,
            ),
          ),
        );
      }

      // 添加高亮文本
      final color = _getHighlightColor(match.keyword);
      spans.add(
        TextSpan(
          text: content.substring(match.index, match.index + match.length),
          style: TextStyle(
            fontFamily: 'FiraCode',
            fontSize: 13,
            backgroundColor: color.withValues(alpha: 0.3),
            color: color,
            fontWeight: FontWeight.bold,
          ),
        ),
      );

      currentIndex = match.index + match.length;
    }

    // 添加剩余文本
    if (currentIndex < content.length) {
      spans.add(
        TextSpan(
          text: content.substring(currentIndex),
          style: const TextStyle(
            fontFamily: 'FiraCode',
            fontSize: 13,
            color: AppColors.textSecondary,
          ),
        ),
      );
    }

    return spans;
  }

  /// 根据关键词获取高亮颜色
  Color _getHighlightColor(String keyword) {
    final colors = [
      AppColors.keywordBlue,
      AppColors.keywordGreen,
      AppColors.keywordRed,
      AppColors.keywordOrange,
      AppColors.keywordPurple,
    ];
    final hash = keyword.hashCode.abs();
    return colors[hash % colors.length];
  }
}

/// 匹配位置信息
class _MatchPosition {
  final String keyword;
  final int index;
  final int length;

  _MatchPosition({
    required this.keyword,
    required this.index,
    required this.length,
  });
}

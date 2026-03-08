import 'package:flutter/material.dart';

/// 压缩包文件预览面板（支持关键词高亮）
class ArchivePreviewPanel extends StatelessWidget {
  final String? content;
  final String searchKeyword;
  final bool isLoading;
  final String? error;
  final bool truncated;
  final String? selectedFileName;

  const ArchivePreviewPanel({
    super.key,
    this.content,
    this.searchKeyword = '',
    this.isLoading = false,
    this.error,
    this.truncated = false,
    this.selectedFileName,
  });

  @override
  Widget build(BuildContext context) {
    // 加载状态
    if (isLoading) {
      return const Center(child: CircularProgressIndicator());
    }

    // 错误状态
    if (error != null) {
      return Center(
        child: Column(
          mainAxisAlignment: MainAxisAlignment.center,
          children: [
            Icon(Icons.error_outline, size: 48, color: Colors.red.shade300),
            const SizedBox(height: 16),
            Text(error!, style: TextStyle(color: Colors.red.shade700)),
          ],
        ),
      );
    }

    // 空状态
    if (content == null || content!.isEmpty) {
      return Center(
        child: Column(
          mainAxisAlignment: MainAxisAlignment.center,
          children: [
            Icon(Icons.info_outline, size: 48, color: Colors.grey.shade400),
            const SizedBox(height: 16),
            Text('选择一个文件预览', style: TextStyle(color: Colors.grey.shade600)),
          ],
        ),
      );
    }

    // 内容显示（支持关键词高亮）
    return Column(
      children: [
        // 截断提示
        if (truncated)
          Container(
            width: double.infinity,
            padding: const EdgeInsets.all(8),
            color: Colors.orange.shade100,
            child: Row(
              children: [
                Icon(
                  Icons.warning_amber,
                  color: Colors.orange.shade700,
                  size: 20,
                ),
                const SizedBox(width: 8),
                Expanded(
                  child: Text(
                    '文件过大，已截断显示',
                    style: TextStyle(color: Colors.orange.shade700),
                  ),
                ),
              ],
            ),
          ),
        // 预览内容
        Expanded(
          child: SingleChildScrollView(
            padding: const EdgeInsets.all(16),
            child: SelectableText.rich(
              _buildHighlightedText(context, content!, searchKeyword),
            ),
          ),
        ),
      ],
    );
  }

  /// 构建带关键词高亮的文本
  TextSpan _buildHighlightedText(
    BuildContext context,
    String text,
    String keyword,
  ) {
    if (keyword.isEmpty) {
      return TextSpan(text: text);
    }

    final spans = <TextSpan>[];
    final regex = RegExp(RegExp.escape(keyword), caseSensitive: false);
    int lastEnd = 0;

    for (final match in regex.allMatches(text)) {
      if (match.start > lastEnd) {
        spans.add(TextSpan(text: text.substring(lastEnd, match.start)));
      }
      spans.add(
        TextSpan(
          text: text.substring(match.start, match.end),
          style: const TextStyle(
            backgroundColor: Colors.yellow,
            fontWeight: FontWeight.bold,
          ),
        ),
      );
      lastEnd = match.end;
    }

    if (lastEnd < text.length) {
      spans.add(TextSpan(text: text.substring(lastEnd)));
    }

    return TextSpan(children: spans);
  }
}

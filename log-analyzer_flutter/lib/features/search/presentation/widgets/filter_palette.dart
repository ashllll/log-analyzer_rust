import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../../../../shared/models/common.dart';
import '../../../../core/theme/app_theme.dart';

/// 过滤器应用回调类型
typedef FilterApplyCallback =
    void Function({
      required TimeRange timeRange,
      required List<String> levels,
      String? filePattern,
    });

/// 过滤器面板组件
///
/// 对应 React 版本的 FilterPalette
/// 功能：
/// - 时间范围选择
/// - 日志级别过滤
/// - 文件模式匹配
class FilterPalette extends ConsumerStatefulWidget {
  /// 应用过滤器回调
  final FilterApplyCallback? onApply;

  /// 当前过滤器配置
  final FilterOptions? currentFilters;

  const FilterPalette({super.key, this.onApply, this.currentFilters});

  @override
  ConsumerState<FilterPalette> createState() => _FilterPaletteState();
}

class _FilterPaletteState extends ConsumerState<FilterPalette> {
  bool _isExpanded = false;

  final _timeStartController = TextEditingController();
  final _timeEndController = TextEditingController();
  final _filePatternController = TextEditingController();

  final Set<String> _selectedLevels = {};

  // 可用的日志级别
  static const availableLevels = [
    'TRACE',
    'DEBUG',
    'INFO',
    'WARN',
    'WARNING',
    'ERROR',
    'FATAL',
  ];

  @override
  void initState() {
    super.initState();
    _initializeFromCurrentFilters();
  }

  // 格式化日期为字符串
  String _formatDate(DateTime date) {
    return '${date.year}-${date.month.toString().padLeft(2, '0')}-${date.day.toString().padLeft(2, '0')}';
  }

  /// 显示日期范围选择器
  Future<void> _selectDateRange() async {
    // 解析当前输入的日期时间
    DateTime? initialStart;
    DateTime? initialEnd;

    if (_timeStartController.text.isNotEmpty) {
      try {
        initialStart = DateTime.parse(_timeStartController.text);
      } catch (_) {}
    }

    if (_timeEndController.text.isNotEmpty) {
      try {
        initialEnd = DateTime.parse(_timeEndController.text);
      } catch (_) {}
    }

    // 设置初始日期范围
    final initialDateRange = (initialStart != null && initialEnd != null)
        ? DateTimeRange(start: initialStart, end: initialEnd)
        : DateTimeRange(
            start: DateTime.now().subtract(const Duration(days: 7)),
            end: DateTime.now(),
          );

    final selectedRange = await showDateRangePicker(
      context: context,
      firstDate: DateTime(2020),
      lastDate: DateTime.now(),
      initialDateRange: initialDateRange,
      builder: (context, child) {
        return Theme(
          data: Theme.of(context).copyWith(
            colorScheme: const ColorScheme.dark(
              primary: AppColors.primary,
              onPrimary: Colors.white,
              surface: AppColors.bgCard,
              onSurface: AppColors.textPrimary,
            ),
          ),
          child: child!,
        );
      },
    );

    if (selectedRange != null) {
      setState(() {
        // 格式化为 YYYY-MM-DD HH:MM:SS
        _timeStartController.text =
            '${_formatDate(selectedRange.start)} 00:00:00';
        _timeEndController.text = '${_formatDate(selectedRange.end)} 23:59:59';
      });
    }
  }

  /// 从当前过滤器初始化状态
  void _initializeFromCurrentFilters() {
    final filters = widget.currentFilters;
    if (filters == null) return;

    // 恢复时间范围
    if (filters.timeRange.start != null) {
      _timeStartController.text = filters.timeRange.start!;
    }
    if (filters.timeRange.end != null) {
      _timeEndController.text = filters.timeRange.end!;
    }

    // 恢复选中的级别
    _selectedLevels.addAll(filters.levels);

    // 恢复文件模式
    if (filters.filePattern != null) {
      _filePatternController.text = filters.filePattern!;
    }
  }

  @override
  void dispose() {
    _timeStartController.dispose();
    _timeEndController.dispose();
    _filePatternController.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    return Column(
      mainAxisSize: MainAxisSize.min,
      children: [
        // 切换按钮
        InkWell(
          onTap: () {
            setState(() {
              _isExpanded = !_isExpanded;
            });
          },
          child: Container(
            padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 12),
            decoration: const BoxDecoration(
              border: Border(
                bottom: BorderSide(color: AppColors.border, width: 1),
              ),
            ),
            child: Row(
              children: [
                Icon(
                  _isExpanded ? Icons.expand_less : Icons.expand_more,
                  size: 20,
                  color: AppColors.textMuted,
                ),
                const SizedBox(width: 8),
                const Text(
                  '高级过滤器',
                  style: TextStyle(
                    color: AppColors.textSecondary,
                    fontSize: 14,
                  ),
                ),
                const Spacer(),
                if (_hasActiveFilters())
                  Container(
                    padding: const EdgeInsets.symmetric(
                      horizontal: 8,
                      vertical: 2,
                    ),
                    decoration: BoxDecoration(
                      color: AppColors.primary.withValues(alpha: 0.2),
                      borderRadius: BorderRadius.circular(10),
                    ),
                    child: Text(
                      '${_selectedLevels.length + (_timeStartController.text.isNotEmpty || _timeEndController.text.isNotEmpty ? 1 : 0)}',
                      style: const TextStyle(
                        color: AppColors.primary,
                        fontSize: 11,
                        fontWeight: FontWeight.w600,
                      ),
                    ),
                  ),
              ],
            ),
          ),
        ),
        // 展开的过滤器内容
        if (_isExpanded)
          Container(
            padding: const EdgeInsets.all(16),
            color: AppColors.bgCard,
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                _buildTimeRangeSection(),
                const SizedBox(height: 16),
                _buildLevelsSection(),
                const SizedBox(height: 16),
                _buildFilePatternSection(),
                const SizedBox(height: 16),
                _buildActions(),
              ],
            ),
          ),
      ],
    );
  }

  /// 构建时间范围部分
  Widget _buildTimeRangeSection() {
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        Row(
          children: [
            const Text(
              '时间范围',
              style: TextStyle(
                color: AppColors.textSecondary,
                fontSize: 13,
                fontWeight: FontWeight.w600,
              ),
            ),
            const Spacer(),
            TextButton.icon(
              onPressed: _selectDateRange,
              icon: const Icon(Icons.calendar_today, size: 16),
              label: const Text('选择日期范围'),
              style: TextButton.styleFrom(
                foregroundColor: AppColors.primary,
                padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 4),
                textStyle: const TextStyle(fontSize: 12),
              ),
            ),
          ],
        ),
        const SizedBox(height: 8),
        Row(
          children: [
            Expanded(
              child: InkWell(
                onTap: _selectDateRange,
                borderRadius: BorderRadius.circular(4),
                child: TextField(
                  controller: _timeStartController,
                  readOnly: true,
                  decoration: InputDecoration(
                    labelText: '开始时间',
                    hintText: '点击选择日期',
                    border: const OutlineInputBorder(),
                    contentPadding: const EdgeInsets.symmetric(
                      horizontal: 12,
                      vertical: 8,
                    ),
                    suffixIcon: IconButton(
                      icon: const Icon(Icons.calendar_today, size: 18),
                      onPressed: _selectDateRange,
                      tooltip: '选择日期',
                    ),
                  ),
                  style: const TextStyle(fontSize: 13),
                  onTap: _selectDateRange,
                ),
              ),
            ),
            const SizedBox(width: 12),
            const Text(
              '至',
              style: TextStyle(color: AppColors.textMuted, fontSize: 13),
            ),
            const SizedBox(width: 12),
            Expanded(
              child: InkWell(
                onTap: _selectDateRange,
                borderRadius: BorderRadius.circular(4),
                child: TextField(
                  controller: _timeEndController,
                  readOnly: true,
                  decoration: InputDecoration(
                    labelText: '结束时间',
                    hintText: '点击选择日期',
                    border: const OutlineInputBorder(),
                    contentPadding: const EdgeInsets.symmetric(
                      horizontal: 12,
                      vertical: 8,
                    ),
                    suffixIcon: IconButton(
                      icon: const Icon(Icons.calendar_today, size: 18),
                      onPressed: _selectDateRange,
                      tooltip: '选择日期',
                    ),
                  ),
                  style: const TextStyle(fontSize: 13),
                  onTap: _selectDateRange,
                ),
              ),
            ),
          ],
        ),
      ],
    );
  }

  /// 构建日志级别部分
  Widget _buildLevelsSection() {
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        const Text(
          '日志级别',
          style: TextStyle(
            color: AppColors.textSecondary,
            fontSize: 13,
            fontWeight: FontWeight.w600,
          ),
        ),
        const SizedBox(height: 8),
        Wrap(
          spacing: 8,
          runSpacing: 8,
          children: availableLevels.map((level) {
            final isSelected = _selectedLevels.contains(level);
            final color = _getLevelColor(level);
            return FilterChip(
              label: Text(
                level,
                style: TextStyle(
                  color: isSelected ? Colors.white : color,
                  fontSize: 12,
                  fontWeight: FontWeight.w500,
                ),
              ),
              selected: isSelected,
              onSelected: (selected) {
                setState(() {
                  if (selected) {
                    _selectedLevels.add(level);
                  } else {
                    _selectedLevels.remove(level);
                  }
                });
              },
              backgroundColor: isSelected
                  ? color.withValues(alpha: 0.9)
                  : color.withValues(alpha: 0.1),
              side: BorderSide(color: color.withValues(alpha: 0.3)),
              shape: const StadiumBorder(),
              padding: const EdgeInsets.symmetric(horizontal: 10, vertical: 6),
            );
          }).toList(),
        ),
      ],
    );
  }

  /// 构建文件模式部分
  Widget _buildFilePatternSection() {
    return TextField(
      controller: _filePatternController,
      decoration: const InputDecoration(
        labelText: '文件模式',
        hintText: '*.log, **/*.txt',
        prefixIcon: Icon(Icons.insert_drive_file_outlined),
        border: OutlineInputBorder(),
        contentPadding: EdgeInsets.symmetric(horizontal: 12, vertical: 8),
      ),
      style: const TextStyle(fontSize: 13),
    );
  }

  /// 构建操作按钮
  Widget _buildActions() {
    return Row(
      children: [
        Expanded(
          child: OutlinedButton(
            onPressed: _clearFilters,
            style: OutlinedButton.styleFrom(
              foregroundColor: AppColors.textSecondary,
              padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 12),
            ),
            child: const Text('清除'),
          ),
        ),
        const SizedBox(width: 12),
        ElevatedButton(
          onPressed: _applyFilters,
          style: ElevatedButton.styleFrom(
            backgroundColor: AppColors.primary,
            foregroundColor: Colors.white,
            padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 12),
          ),
          child: const Text('应用'),
        ),
      ],
    );
  }

  /// 检查是否有活动的过滤器
  bool _hasActiveFilters() {
    return _selectedLevels.isNotEmpty ||
        _timeStartController.text.isNotEmpty ||
        _timeEndController.text.isNotEmpty;
  }

  /// 清除过滤器
  void _clearFilters() {
    setState(() {
      _selectedLevels.clear();
      _timeStartController.clear();
      _timeEndController.clear();
      _filePatternController.clear();
    });
  }

  /// 应用过滤器
  ///
  /// 构建过滤器配置并回调到父组件
  void _applyFilters() {
    // 构建时间范围
    final timeRange = TimeRange(
      start: _timeStartController.text.trim().isNotEmpty
          ? _timeStartController.text.trim()
          : null,
      end: _timeEndController.text.trim().isNotEmpty
          ? _timeEndController.text.trim()
          : null,
    );

    // 获取选中的级别列表
    final levels = _selectedLevels.toList();

    // 获取文件模式
    final filePattern = _filePatternController.text.trim().isNotEmpty
        ? _filePatternController.text.trim()
        : null;

    // 通过回调传递过滤器配置
    widget.onApply?.call(
      timeRange: timeRange,
      levels: levels,
      filePattern: filePattern,
    );

    // 折叠过滤器面板
    setState(() {
      _isExpanded = false;
    });
  }

  /// 获取日志级别颜色
  Color _getLevelColor(String level) {
    switch (level) {
      case 'ERROR':
      case 'FATAL':
        return AppColors.error;
      case 'WARN':
      case 'WARNING':
        return AppColors.warning;
      case 'INFO':
        return AppColors.primary;
      case 'DEBUG':
        return AppColors.keywordPurple;
      default:
        return AppColors.textMuted;
    }
  }
}

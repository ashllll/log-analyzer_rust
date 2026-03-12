import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../../../../shared/models/common.dart' as common;
import '../../../../shared/models/saved_filter.dart' as saved;
import '../../../../shared/providers/saved_filters_provider.dart';
import '../../../../core/theme/app_theme.dart';
import 'filter_palette.dart';

/// 使用 common.dart 的类型别名
typedef TimeRange = common.TimeRange;
typedef FilterOptions = common.FilterOptions;

/// 过滤器编辑器结果
class FilterEditorResult {
  final String name;
  final String? description;
  final TimeRange timeRange;
  final List<String> levels;
  final String? filePattern;
  final bool isDefault;

  FilterEditorResult({
    required this.name,
    this.description,
    required this.timeRange,
    required this.levels,
    this.filePattern,
    this.isDefault = false,
  });
}

/// 过滤器编辑对话框
///
/// 支持创建新过滤器和编辑现有过滤器
class FilterEditorDialog extends ConsumerStatefulWidget {
  /// 工作区ID
  final String workspaceId;

  /// 要编辑的过滤器（null 表示创建新过滤器）
  final saved.SavedFilter? filter;

  /// 当前的过滤器配置（用于初始化）
  final FilterOptions? currentFilters;

  const FilterEditorDialog({
    super.key,
    required this.workspaceId,
    this.filter,
    this.currentFilters,
  });

  /// 显示过滤器编辑器对话框
  ///
  /// 返回 FilterEditorResult 或 null（取消）
  static Future<FilterEditorResult?> show(
    BuildContext context, {
    required String workspaceId,
    saved.SavedFilter? filter,
    FilterOptions? currentFilters,
  }) {
    return showDialog<FilterEditorResult>(
      context: context,
      builder: (context) => FilterEditorDialog(
        workspaceId: workspaceId,
        filter: filter,
        currentFilters: currentFilters,
      ),
    );
  }

  @override
  ConsumerState<FilterEditorDialog> createState() => _FilterEditorDialogState();
}

class _FilterEditorDialogState extends ConsumerState<FilterEditorDialog> {
  late final TextEditingController _nameController;
  late final TextEditingController _descriptionController;
  late final TextEditingController _filePatternController;

  // 时间范围
  DateTime? _startDate;
  DateTime? _endDate;

  // 选中的日志级别
  final Set<String> _selectedLevels = {};

  // 是否设为默认
  bool _isDefault = false;

  // 表单验证
  bool _isValid = false;
  String? _nameError;

  // 保存中状态
  bool _isSaving = false;

  @override
  void initState() {
    super.initState();
    _initFromFilter();
    _nameController = TextEditingController(text: widget.filter?.name ?? '');
    _descriptionController =
        TextEditingController(text: widget.filter?.description ?? '');
    _filePatternController =
        TextEditingController(text: widget.filter?.filePattern ?? '');
    _nameController.addListener(_validateForm);
    _validateForm();
  }

  /// 从现有过滤器初始化
  void _initFromFilter() {
    final filter = widget.filter;
    if (filter != null) {
      // 从现有过滤器加载
      _isDefault = filter.isDefault;

      // 加载时间范围
      if (filter.timeRange != null) {
        if (filter.timeRange!.start != null) {
          try {
            _startDate = DateTime.parse(filter.timeRange!.start!);
          } catch (_) {}
        }
        if (filter.timeRange!.end != null) {
          try {
            _endDate = DateTime.parse(filter.timeRange!.end!);
          } catch (_) {}
        }
      }

      // 加载级别
      _selectedLevels.addAll(filter.levels);
    } else if (widget.currentFilters != null) {
      // 从当前过滤器配置加载
      final filters = widget.currentFilters!;
      _selectedLevels.addAll(filters.levels);

      if (filters.timeRange.start != null) {
        try {
          _startDate = DateTime.parse(filters.timeRange.start!);
        } catch (_) {}
      }
      if (filters.timeRange.end != null) {
        try {
          _endDate = DateTime.parse(filters.timeRange.end!);
        } catch (_) {}
      }

      if (filters.filePattern != null) {
        _filePatternController.text = filters.filePattern!;
      }
    }
  }

  @override
  void dispose() {
    _nameController.dispose();
    _descriptionController.dispose();
    _filePatternController.dispose();
    super.dispose();
  }

  /// 验证表单
  void _validateForm() {
    final name = _nameController.text.trim();
    setState(() {
      if (name.isEmpty) {
        _nameError = '请输入过滤器名称';
        _isValid = false;
      } else if (name.length > 50) {
        _nameError = '名称不能超过50个字符';
        _isValid = false;
      } else {
        _nameError = null;
        _isValid = true;
      }
    });
  }

  /// 格式化日期为字符串
  String _formatDate(DateTime date) {
    return '${date.year}-${date.month.toString().padLeft(2, '0')}-${date.day.toString().padLeft(2, '0')}';
  }

  /// 构建时间范围显示
  String _buildTimeRangeText() {
    if (_startDate == null && _endDate == null) {
      return '未设置';
    }
    if (_startDate != null && _endDate != null) {
      return '${_formatDate(_startDate!)} 至 ${_formatDate(_endDate!)}';
    }
    if (_startDate != null) {
      return '从 ${_formatDate(_startDate!)} 开始';
    }
    return '直到 ${_formatDate(_endDate!)}';
  }

  /// 构建预览摘要
  String _buildPreviewSummary() {
    final parts = <String>[];

    // 关键词（暂无）
    if (widget.currentFilters != null) {
      parts.add('使用当前搜索条件');
    }

    // 级别
    if (_selectedLevels.isNotEmpty) {
      parts.add('级别: ${_selectedLevels.join(", ")}');
    }

    // 时间范围
    if (_startDate != null || _endDate != null) {
      parts.add('时间: ${_buildTimeRangeText()}');
    }

    // 文件模式
    final filePattern = _filePatternController.text.trim();
    if (filePattern.isNotEmpty) {
      parts.add('文件: $filePattern');
    }

    return parts.isEmpty ? '无过滤条件' : parts.join(' | ');
  }

  /// 保存过滤器
  Future<void> _saveFilter() async {
    if (!_isValid || _isSaving) return;

    setState(() {
      _isSaving = true;
    });

    try {
      // 构建时间范围 (使用 saved_filter.dart 的 TimeRange)
      saved.TimeRange? timeRange;
      if (_startDate != null || _endDate != null) {
        timeRange = saved.TimeRange(
          start: _startDate != null
              ? '${_formatDate(_startDate!)} 00:00:00'
              : null,
          end: _endDate != null
              ? '${_formatDate(_endDate!)} 23:59:59'
              : null,
        );
      }

      // 文件模式
      final filePattern = _filePatternController.text.trim();
      final filePatternValue = filePattern.isNotEmpty ? filePattern : null;

      // 构建过滤器 (使用 saved_filter.dart 的 SavedFilter)
      final filter = saved.SavedFilter(
        id: widget.filter?.id ?? '',
        name: _nameController.text.trim(),
        description: _descriptionController.text.trim().isNotEmpty
            ? _descriptionController.text.trim()
            : null,
        workspaceId: widget.workspaceId,
        terms: widget.filter?.terms ?? [],
        globalOperator: widget.filter?.globalOperator ?? 'AND',
        timeRange: timeRange,
        levels: _selectedLevels.toList(),
        filePattern: filePatternValue,
        isDefault: _isDefault,
        sortOrder: widget.filter?.sortOrder ?? 0,
        usageCount: widget.filter?.usageCount ?? 0,
        createdAt: widget.filter?.createdAt ?? DateTime.now().toIso8601String(),
        lastUsedAt: widget.filter?.lastUsedAt,
      );

      // 保存过滤器
      final success = await ref
          .read(savedFiltersProvider(widget.workspaceId).notifier)
          .saveFilter(filter);

      if (success && mounted) {
        // 转换 saved.TimeRange 到 common.TimeRange
        final commonTimeRange = timeRange != null
            ? TimeRange(start: timeRange.start, end: timeRange.end)
            : const TimeRange();

        // 返回结果
        Navigator.of(context).pop(FilterEditorResult(
          name: filter.name,
          description: filter.description,
          timeRange: commonTimeRange,
          levels: _selectedLevels.toList(),
          filePattern: filePatternValue,
          isDefault: _isDefault,
        ));
      } else if (mounted) {
        ScaffoldMessenger.of(context).showSnackBar(
          const SnackBar(
            content: Text('保存过滤器失败'),
            backgroundColor: AppColors.error,
          ),
        );
      }
    } catch (e) {
      if (mounted) {
        ScaffoldMessenger.of(context).showSnackBar(
          SnackBar(
            content: Text('保存过滤器失败: $e'),
            backgroundColor: AppColors.error,
          ),
        );
      }
    } finally {
      if (mounted) {
        setState(() {
          _isSaving = false;
        });
      }
    }
  }

  @override
  Widget build(BuildContext context) {
    final isEditing = widget.filter != null;

    return AlertDialog(
      title: Row(
        children: [
          Icon(
            isEditing ? Icons.edit : Icons.add,
            color: AppColors.primary,
          ),
          const SizedBox(width: 8),
          Text(isEditing ? '编辑过滤器' : '创建过滤器'),
        ],
      ),
      content: SizedBox(
        width: 500,
        child: SingleChildScrollView(
          child: Column(
            mainAxisSize: MainAxisSize.min,
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              // 过滤器名称
              _buildNameField(),
              const SizedBox(height: 16),

              // 过滤器描述
              _buildDescriptionField(),
              const SizedBox(height: 20),

              // 过滤条件
              _buildFilterSection(),
              const SizedBox(height: 20),

              // 设为默认
              _buildDefaultCheckbox(),
              const SizedBox(height: 16),

              // 预览
              _buildPreview(),
            ],
          ),
        ),
      ),
      actions: [
        TextButton(
          onPressed: () => Navigator.of(context).pop(),
          child: const Text('取消'),
        ),
        ElevatedButton(
          onPressed: _isValid && !_isSaving ? _saveFilter : null,
          style: ElevatedButton.styleFrom(
            backgroundColor: AppColors.primary,
            foregroundColor: Colors.white,
          ),
          child: _isSaving
              ? const SizedBox(
                  width: 16,
                  height: 16,
                  child: CircularProgressIndicator(
                    strokeWidth: 2,
                    valueColor: AlwaysStoppedAnimation<Color>(Colors.white),
                  ),
                )
              : Text(isEditing ? '保存' : '创建'),
        ),
      ],
    );
  }

  /// 构建名称输入字段
  Widget _buildNameField() {
    return TextField(
      controller: _nameController,
      decoration: InputDecoration(
        labelText: '过滤器名称 *',
        hintText: '输入过滤器名称（最多50个字符）',
        errorText: _nameError,
        border: const OutlineInputBorder(),
        prefixIcon: const Icon(Icons.label_outline),
      ),
      maxLength: 50,
      textInputAction: TextInputAction.next,
    );
  }

  /// 构建描述输入字段
  Widget _buildDescriptionField() {
    return TextField(
      controller: _descriptionController,
      decoration: const InputDecoration(
        labelText: '描述（可选）',
        hintText: '输入过滤器描述（最多200个字符）',
        border: OutlineInputBorder(),
        prefixIcon: Icon(Icons.description_outlined),
      ),
      maxLength: 200,
      maxLines: 2,
    );
  }

  /// 构建过滤条件部分
  Widget _buildFilterSection() {
    // 构建当前过滤器配置用于初始化 FilterPalette
    final currentFilters = FilterOptions(
      timeRange: TimeRange(
        start: _startDate != null
            ? '${_formatDate(_startDate!)} 00:00:00'
            : null,
        end: _endDate != null
            ? '${_formatDate(_endDate!)} 23:59:59'
            : null,
      ),
      levels: _selectedLevels.toList(),
      filePattern: _filePatternController.text.trim().isNotEmpty
          ? _filePatternController.text.trim()
          : null,
    );

    return Container(
      padding: const EdgeInsets.all(16),
      decoration: BoxDecoration(
        color: AppColors.bgCard,
        borderRadius: BorderRadius.circular(8),
        border: Border.all(color: AppColors.border),
      ),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          const Text(
            '过滤条件',
            style: TextStyle(
              color: AppColors.textPrimary,
              fontSize: 14,
              fontWeight: FontWeight.w600,
            ),
          ),
          const SizedBox(height: 16),

          // 使用 FilterPalette 组件
          FilterPalette(
            currentFilters: currentFilters,
            onApply: ({
              required TimeRange timeRange,
              required List<String> levels,
              String? filePattern,
            }) {
              // 同步 FilterPalette 的状态回对话框
              setState(() {
                // 解析时间范围
                if (timeRange.start != null) {
                  try {
                    _startDate = DateTime.parse(timeRange.start!);
                  } catch (_) {
                    _startDate = null;
                  }
                } else {
                  _startDate = null;
                }

                if (timeRange.end != null) {
                  try {
                    _endDate = DateTime.parse(timeRange.end!);
                  } catch (_) {
                    _endDate = null;
                  }
                } else {
                  _endDate = null;
                }

                // 更新级别
                _selectedLevels.clear();
                _selectedLevels.addAll(levels);

                // 更新文件模式
                _filePatternController.text = filePattern ?? '';
              });
            },
          ),
        ],
      ),
    );
  }

  /// 构建设为默认复选框
  Widget _buildDefaultCheckbox() {
    return CheckboxListTile(
      value: _isDefault,
      onChanged: (value) {
        setState(() {
          _isDefault = value ?? false;
        });
      },
      title: const Text(
        '设为默认过滤器',
        style: TextStyle(
          color: AppColors.textPrimary,
          fontSize: 14,
        ),
      ),
      subtitle: const Text(
        '默认过滤器将在打开过滤器列表时自动应用',
        style: TextStyle(
          color: AppColors.textMuted,
          fontSize: 12,
        ),
      ),
      controlAffinity: ListTileControlAffinity.leading,
      contentPadding: EdgeInsets.zero,
    );
  }

  /// 构建预览部分
  Widget _buildPreview() {
    return Container(
      padding: const EdgeInsets.all(12),
      decoration: BoxDecoration(
        color: AppColors.bgMain,
        borderRadius: BorderRadius.circular(8),
        border: Border.all(color: AppColors.border),
      ),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          const Row(
            children: [
              Icon(Icons.preview, size: 16, color: AppColors.textMuted),
              SizedBox(width: 8),
              Text(
                '预览',
                style: TextStyle(
                  color: AppColors.textSecondary,
                  fontSize: 12,
                  fontWeight: FontWeight.w500,
                ),
              ),
            ],
          ),
          const SizedBox(height: 8),
          Text(
            _buildPreviewSummary(),
            style: const TextStyle(
              color: AppColors.textPrimary,
              fontSize: 13,
            ),
          ),
        ],
      ),
    );
  }

}

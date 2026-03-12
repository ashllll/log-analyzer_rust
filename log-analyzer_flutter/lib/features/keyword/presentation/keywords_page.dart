import 'dart:async';
import 'dart:convert';
import 'dart:io';

import 'package:file_picker/file_picker.dart';
import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../../../shared/models/keyword.dart';
import '../../../shared/providers/keyword_provider.dart';
import '../../../shared/providers/app_provider.dart';
import '../../../core/theme/app_theme.dart';
import '../../../core/constants/app_constants.dart';

/// 关键词管理页面
///
/// 对应 React 版本的 KeywordsPage.tsx
/// 功能：
/// - 关键词组 CRUD
/// - 颜色选择器
/// - 启用/禁用切换
/// - 导入/导出配置
class KeywordsPage extends ConsumerStatefulWidget {
  const KeywordsPage({super.key});

  @override
  ConsumerState<KeywordsPage> createState() => _KeywordsPageState();
}

class _KeywordsPageState extends ConsumerState<KeywordsPage> {
  @override
  Widget build(BuildContext context) {
    final keywordGroups = ref.watch(keywordStateProvider);
    // 从 provider 获取加载状态
    final isLoading = ref.watch(keywordLoadingProvider);

    return Scaffold(
      appBar: _buildAppBar(context),
      body: isLoading
          ? _buildLoadingState()
          : keywordGroups.isEmpty
          ? _buildEmptyState(context)
          : _buildKeywordList(keywordGroups),
      floatingActionButton: FloatingActionButton(
        onPressed: () => _showKeywordGroupDialog(context),
        backgroundColor: AppColors.primary,
        child: const Icon(Icons.add),
      ),
    );
  }

  /// 构建 AppBar
  PreferredSizeWidget _buildAppBar(BuildContext context) {
    return AppBar(
      backgroundColor: AppColors.bgMain,
      elevation: 0,
      title: const Text(
        '关键词',
        style: TextStyle(fontSize: 18, fontWeight: FontWeight.w600),
      ),
      actions: [
        // 导入按钮
        IconButton(
          icon: const Icon(Icons.file_upload_outlined),
          tooltip: '导入配置',
          onPressed: () => _importKeywords(context),
        ),
        // 导出按钮
        IconButton(
          icon: const Icon(Icons.file_download_outlined),
          tooltip: '导出配置',
          onPressed: () => _exportKeywords(context),
        ),
      ],
    );
  }

  /// 构建加载状态
  Widget _buildLoadingState() {
    return const Center(child: CircularProgressIndicator());
  }

  /// 构建空状态
  Widget _buildEmptyState(BuildContext context) {
    return Center(
      child: Column(
        mainAxisAlignment: MainAxisAlignment.center,
        children: [
          const Icon(
            Icons.label_outlined,
            size: 64,
            color: AppColors.textMuted,
          ),
          const SizedBox(height: 16),
          const Text(
            '暂无关键词组',
            style: TextStyle(fontSize: 16, color: AppColors.textSecondary),
          ),
          const SizedBox(height: 8),
          ElevatedButton.icon(
            onPressed: () => _showKeywordGroupDialog(context),
            icon: const Icon(Icons.add),
            label: const Text('添加关键词组'),
            style: ElevatedButton.styleFrom(
              backgroundColor: AppColors.primary,
              foregroundColor: Colors.white,
            ),
          ),
        ],
      ),
    );
  }

  /// 构建关键词组列表
  Widget _buildKeywordList(List<KeywordGroup> groups) {
    return ReorderableListView.builder(
      padding: const EdgeInsets.all(16),
      onReorder: (oldIndex, newIndex) {
        // 实现拖拽排序
        ref
            .read(keywordStateProvider.notifier)
            .reorderKeywordGroups(oldIndex, newIndex);
      },
      itemCount: groups.length,
      itemBuilder: (context, index) {
        final group = groups[index];
        return _KeywordGroupCard(
          key: ValueKey(group.id),
          group: group,
          onTap: () => _showKeywordGroupDialog(context, group: group),
          onToggle: () => _toggleKeywordGroup(group),
          onDelete: () => _confirmDeleteKeywordGroup(context, group),
          onEdit: () => _showKeywordGroupDialog(context, group: group),
          onDuplicate: () => _duplicateKeywordGroup(group),
        );
      },
    );
  }

  /// 切换关键词组启用状态
  void _toggleKeywordGroup(KeywordGroup group) {
    ref.read(keywordStateProvider.notifier).toggleKeywordGroup(group.id);
  }

  /// 显示添加/编辑关键词组对话框
  void _showKeywordGroupDialog(BuildContext context, {KeywordGroup? group}) {
    showDialog(
      context: context,
      builder: (context) => _KeywordGroupDialog(
        group: group,
        onSave: (name, color, patterns, enabled) {
          final newGroup = KeywordGroup(
            id: group?.id ?? DateTime.now().millisecondsSinceEpoch.toString(),
            name: name,
            color: ColorKeyData(value: color),
            patterns: patterns,
            enabled: enabled,
          );

          if (group == null) {
            ref.read(keywordStateProvider.notifier).addKeywordGroup(newGroup);
          } else {
            ref
                .read(keywordStateProvider.notifier)
                .updateKeywordGroup(newGroup);
          }
        },
      ),
    );
  }

  /// 确认删除关键词组
  void _confirmDeleteKeywordGroup(BuildContext context, KeywordGroup group) {
    showDialog(
      context: context,
      builder: (dialogContext) => AlertDialog(
        backgroundColor: AppColors.bgCard,
        title: const Text('删除关键词组'),
        content: Text('确定要删除关键词组 "${group.name}" 吗？'),
        actions: [
          TextButton(
            onPressed: () => Navigator.pop(dialogContext),
            child: const Text('取消'),
          ),
          ElevatedButton(
            onPressed: () {
              Navigator.pop(dialogContext);
              _deleteKeywordGroup(group);
            },
            style: ElevatedButton.styleFrom(backgroundColor: AppColors.error),
            child: const Text('删除'),
          ),
        ],
      ),
    );
  }

  /// 删除关键词组
  void _deleteKeywordGroup(KeywordGroup group) {
    // 使用本地操作方法（removeKeywordGroup）
    ref.read(keywordStateProvider.notifier).removeKeywordGroup(group.id);
    ref.read(appStateProvider.notifier).addToast(ToastType.success, '关键词组已删除');
  }

  /// 复制关键词组
  void _duplicateKeywordGroup(KeywordGroup group) {
    ref.read(keywordStateProvider.notifier).duplicateKeywordGroup(group.id);
    ref
        .read(appStateProvider.notifier)
        .addToast(ToastType.success, '关键词组 "${group.name}" 已复制');
  }

  /// 导入关键词配置
  Future<void> _importKeywords(BuildContext context) async {
    try {
      // 使用 file_picker 选择文件
      final result = await FilePicker.platform.pickFiles(
        type: FileType.custom,
        allowedExtensions: ['json'],
        dialogTitle: '选择关键词配置文件',
      );

      if (result == null || result.files.isEmpty) {
        // 用户取消选择
        return;
      }

      final file = result.files.first;
      if (file.path == null) {
        ref
            .read(appStateProvider.notifier)
            .addToast(ToastType.error, '无法获取文件路径');
        return;
      }

      // 读取文件内容
      final fileContent = await _readFileContent(file.path!);

      // 导入关键词组
      final count = ref
          .read(keywordStateProvider.notifier)
          .importFromJson(fileContent);

      ref
          .read(appStateProvider.notifier)
          .addToast(ToastType.success, '成功导入 $count 个关键词组');
    } on FormatException catch (e) {
      ref
          .read(appStateProvider.notifier)
          .addToast(ToastType.error, '导入失败: ${e.message}');
    } catch (e) {
      ref.read(appStateProvider.notifier).addToast(ToastType.error, '导入失败: $e');
    }
  }

  /// 读取文件内容
  Future<String> _readFileContent(String filePath) async {
    final file = File(filePath);
    return file.readAsString();
  }

  /// 导出关键词配置
  Future<void> _exportKeywords(BuildContext context) async {
    final groups = ref.read(keywordStateProvider);

    if (groups.isEmpty) {
      ref
          .read(appStateProvider.notifier)
          .addToast(ToastType.warning, '没有可导出的关键词组');
      return;
    }

    try {
      // 生成 JSON 内容
      final jsonContent = ref
          .read(keywordStateProvider.notifier)
          .exportToJson();

      // 选择保存位置
      final outputPath = await FilePicker.platform.saveFile(
        dialogTitle: '保存关键词配置',
        fileName: 'keywords_${DateTime.now().millisecondsSinceEpoch}.json',
        type: FileType.custom,
        allowedExtensions: ['json'],
        bytes: utf8.encode(jsonContent),
      );

      if (outputPath == null) {
        // 用户取消保存
        return;
      }

      // 如果 bytes 参数不起作用，手动写入文件
      // 某些平台可能需要这种方式
      if (!File(outputPath).existsSync()) {
        final file = File(outputPath);
        await file.writeAsString(jsonContent);
      }

      ref
          .read(appStateProvider.notifier)
          .addToast(ToastType.success, '成功导出 ${groups.length} 个关键词组');
    } catch (e) {
      ref.read(appStateProvider.notifier).addToast(ToastType.error, '导出失败: $e');
    }
  }
}

/// 关键词组卡片组件
class _KeywordGroupCard extends StatelessWidget {
  final KeywordGroup group;
  final VoidCallback onTap;
  final VoidCallback onToggle;
  final VoidCallback onDelete;
  final VoidCallback onEdit;
  final VoidCallback onDuplicate;

  const _KeywordGroupCard({
    super.key,
    required this.group,
    required this.onTap,
    required this.onToggle,
    required this.onDelete,
    required this.onEdit,
    required this.onDuplicate,
  });

  @override
  Widget build(BuildContext context) {
    final color = AppColors.fromColorKey(group.color.value);

    return Card(
      margin: const EdgeInsets.only(bottom: 12),
      child: InkWell(
        onTap: onTap,
        borderRadius: BorderRadius.circular(8),
        child: Container(
          padding: const EdgeInsets.all(16),
          decoration: BoxDecoration(
            border: Border.all(
              color: group.enabled
                  ? color.withOpacity(0.3)
                  : Colors.transparent,
              width: 2,
            ),
            borderRadius: BorderRadius.circular(8),
          ),
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              // 标题行
              Row(
                children: [
                  // 颜色指示器
                  Container(
                    width: 16,
                    height: 16,
                    decoration: BoxDecoration(
                      color: color,
                      borderRadius: BorderRadius.circular(4),
                    ),
                  ),
                  const SizedBox(width: 12),
                  // 名称
                  Expanded(
                    child: Text(
                      group.name,
                      style: TextStyle(
                        fontSize: 16,
                        fontWeight: FontWeight.w600,
                        color: group.enabled
                            ? AppColors.textPrimary
                            : AppColors.textSecondary,
                      ),
                    ),
                  ),
                  // 启用开关
                  Switch(
                    value: group.enabled,
                    onChanged: (_) => onToggle(),
                    activeThumbColor: AppColors.primary,
                  ),
                  // 更多按钮
                  IconButton(
                    icon: const Icon(Icons.more_vert, size: 20),
                    onPressed: () => _showMenu(context),
                    padding: EdgeInsets.zero,
                  ),
                ],
              ),
              const SizedBox(height: 8),
              // 模式列表
              if (group.patterns.isNotEmpty)
                Wrap(
                  spacing: 8,
                  runSpacing: 8,
                  children: [
                    ...group.patterns
                        .take(3)
                        .map((pattern) => _buildPatternChip(pattern, color)),
                    if (group.patterns.length > 3)
                      _buildMoreIndicator(group.patterns.length - 3),
                  ],
                ),
              // 统计信息
              const SizedBox(height: 8),
              Text(
                '${group.patterns.length} 个模式',
                style: const TextStyle(
                  fontSize: 12,
                  color: AppColors.textMuted,
                ),
              ),
            ],
          ),
        ),
      ),
    );
  }

  /// 构建模式标签
  Widget _buildPatternChip(KeywordPattern pattern, Color color) {
    return Container(
      padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 4),
      decoration: BoxDecoration(
        color: color.withOpacity(0.1),
        borderRadius: BorderRadius.circular(4),
        border: Border.all(color: color.withOpacity(0.2), width: 1),
      ),
      child: Row(
        mainAxisSize: MainAxisSize.min,
        children: [
          if (pattern.comment.isNotEmpty)
            Text(
              pattern.comment,
              style: TextStyle(
                color: color.withOpacity(0.8),
                fontSize: 11,
              ),
            ),
          if (pattern.comment.isNotEmpty) const SizedBox(width: 4),
          Text(
            pattern.regex,
            style: TextStyle(
              color: color,
              fontSize: 11,
              fontFamily: 'monospace',
            ),
          ),
        ],
      ),
    );
  }

  /// 构建"更多"指示器
  Widget _buildMoreIndicator(int count) {
    return Container(
      padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 4),
      decoration: BoxDecoration(
        color: AppColors.bgHover,
        borderRadius: BorderRadius.circular(4),
      ),
      child: Text(
        '+$count',
        style: const TextStyle(
          color: AppColors.textMuted,
          fontSize: 11,
          fontWeight: FontWeight.w500,
        ),
      ),
    );
  }

  /// 显示菜单
  void _showMenu(BuildContext context) {
    showModalBottomSheet(
      context: context,
      backgroundColor: Colors.transparent,
      builder: (context) => Container(
        decoration: const BoxDecoration(
          color: AppColors.bgCard,
          borderRadius: BorderRadius.vertical(top: Radius.circular(16)),
        ),
        child: SafeArea(
          child: Column(
            mainAxisSize: MainAxisSize.min,
            children: [
              ListTile(
                leading: const Icon(Icons.edit, size: 20),
                title: const Text('编辑'),
                onTap: () {
                  Navigator.pop(context);
                  onEdit();
                },
              ),
              ListTile(
                leading: const Icon(Icons.copy, size: 20),
                title: const Text('复制'),
                onTap: () {
                  Navigator.pop(context);
                  onDuplicate();
                },
              ),
              ListTile(
                leading: const Icon(Icons.delete_outline, size: 20),
                title: const Text(
                  '删除',
                  style: TextStyle(color: AppColors.error),
                ),
                onTap: () {
                  Navigator.pop(context);
                  onDelete();
                },
              ),
            ],
          ),
        ),
      ),
    );
  }
}

/// 关键词组对话框
class _KeywordGroupDialog extends ConsumerStatefulWidget {
  final KeywordGroup? group;
  final Function(
    String name,
    String color,
    List<KeywordPattern> patterns,
    bool enabled,
  )
  onSave;

  const _KeywordGroupDialog({this.group, required this.onSave});

  @override
  ConsumerState<_KeywordGroupDialog> createState() =>
      _KeywordGroupDialogState();
}

class _KeywordGroupDialogState extends ConsumerState<_KeywordGroupDialog> {
  final _nameController = TextEditingController();
  final _patternController = TextEditingController();

  final _formKey = GlobalKey<FormState>();
  final List<_PatternItem> _patterns = [];

  String _selectedColor = 'blue';
  bool _enabled = true;

  @override
  void initState() {
    super.initState();
    if (widget.group != null) {
      _nameController.text = widget.group!.name;
      _selectedColor = widget.group!.color.value;
      _enabled = widget.group!.enabled;
      _patterns.addAll(
        widget.group!.patterns.map(
          (p) => _PatternItem(regex: p.regex, comment: p.comment),
        ),
      );
    }
  }

  @override
  void dispose() {
    _nameController.dispose();
    _patternController.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    return AlertDialog(
      backgroundColor: AppColors.bgCard,
      title: Text(widget.group == null ? '添加关键词组' : '编辑关键词组'),
      content: SizedBox(
        width: 500,
        child: Form(
          key: _formKey,
          child: Column(
            mainAxisSize: MainAxisSize.min,
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              // 名称输入
              TextFormField(
                controller: _nameController,
                decoration: const InputDecoration(
                  labelText: '组名称',
                  hintText: '例如: 错误关键词',
                  border: OutlineInputBorder(),
                ),
                validator: (value) {
                  if (value == null || value.isEmpty) {
                    return '请输入组名称';
                  }
                  return null;
                },
              ),
              const SizedBox(height: 16),

              // 颜色选择
              const Text(
                '颜色',
                style: TextStyle(fontSize: 14, fontWeight: FontWeight.w500),
              ),
              const SizedBox(height: 8),
              Wrap(
                spacing: 8,
                children: ColorKey.values.map((color) {
                  return ChoiceChip(
                    label: Text(color.name),
                    selected: _selectedColor == color.name,
                    onSelected: (_) {
                      setState(() {
                        _selectedColor = color.name;
                      });
                    },
                    selectedColor: AppColors.fromColorKey(color.name),
                    labelStyle: TextStyle(
                      color: _selectedColor == color.name
                          ? Colors.white
                          : AppColors.textSecondary,
                      fontWeight: FontWeight.w500,
                    ),
                    shape: const StadiumBorder(),
                  );
                }).toList(),
              ),
              const SizedBox(height: 16),

              // 启用开关
              Row(
                children: [
                  const Text('启用此关键词组', style: TextStyle(fontSize: 14)),
                  const Spacer(),
                  Switch(
                    value: _enabled,
                    onChanged: (value) {
                      setState(() {
                        _enabled = value;
                      });
                    },
                    activeThumbColor: AppColors.primary,
                  ),
                ],
              ),
              const SizedBox(height: 16),

              // 模式列表
              const Text(
                '匹配模式',
                style: TextStyle(fontSize: 14, fontWeight: FontWeight.w500),
              ),
              const SizedBox(height: 8),
              Row(
                children: [
                  Expanded(
                    child: TextFormField(
                      controller: _patternController,
                      decoration: const InputDecoration(
                        labelText: '正则表达式',
                        hintText: '例如: \\berror\\b',
                        border: OutlineInputBorder(),
                      ),
                      onFieldSubmitted: (value) => _addPattern(value),
                    ),
                  ),
                  IconButton(
                    icon: const Icon(Icons.add),
                    onPressed: () => _addPattern(_patternController.text),
                  ),
                ],
              ),
              const SizedBox(height: 16),

              // 已添加的模式列表
              const Text(
                '已添加的模式',
                style: TextStyle(fontSize: 12, fontWeight: FontWeight.w500),
              ),
              const SizedBox(height: 8),
              Container(
                constraints: const BoxConstraints(maxHeight: 200),
                decoration: BoxDecoration(
                  color: AppColors.bgInput,
                  borderRadius: BorderRadius.circular(8),
                  border: Border.all(color: AppColors.border),
                ),
                child: _patterns.isEmpty
                    ? const Padding(
                        padding: EdgeInsets.all(16),
                        child: Text(
                          '暂无模式',
                          style: TextStyle(
                            color: AppColors.textMuted,
                            fontSize: 13,
                          ),
                        ),
                      )
                    : ListView.separated(
                        shrinkWrap: true,
                        itemCount: _patterns.length,
                        separatorBuilder: (_, __) => const Divider(height: 1),
                        itemBuilder: (context, index) {
                          final pattern = _patterns[index];
                          return _PatternListItem(
                            pattern: pattern,
                            onRemove: () => _removePattern(index),
                          );
                        },
                      ),
              ),
            ],
          ),
        ),
      ),
      actions: [
        TextButton(
          onPressed: () => Navigator.pop(context),
          child: const Text('取消'),
        ),
        ElevatedButton(onPressed: _save, child: const Text('保存')),
      ],
    );
  }

  /// 添加模式
  void _addPattern(String regex) {
    if (regex.isEmpty) return;

    setState(() {
      _patterns.add(_PatternItem(regex: regex, comment: ''));
      _patternController.clear();
    });
  }

  /// 移除模式
  void _removePattern(int index) {
    setState(() {
      _patterns.removeAt(index);
    });
  }

  /// 保存
  void _save() {
    if (_formKey.currentState?.validate() ?? false) {
      final name = _nameController.text.trim();
      if (name.isEmpty) return;

      widget.onSave(
        name,
        _selectedColor,
        _patterns
            .map((p) => KeywordPattern(regex: p.regex, comment: p.comment))
            .toList(),
        _enabled,
      );

      Navigator.pop(context);
    }
  }
}

/// 模式项
class _PatternItem {
  final String regex;
  final String comment;

  _PatternItem({required this.regex, this.comment = ''});
}

/// 模式列表项
class _PatternListItem extends StatelessWidget {
  final _PatternItem pattern;
  final VoidCallback onRemove;

  const _PatternListItem({required this.pattern, required this.onRemove});

  @override
  Widget build(BuildContext context) {
    return Padding(
      padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 4),
      child: Row(
        children: [
          Expanded(
            child: Text(
              pattern.regex,
              style: const TextStyle(fontSize: 13, fontFamily: 'monospace'),
            ),
          ),
          IconButton(
            icon: const Icon(Icons.close, size: 16),
            onPressed: onRemove,
            padding: EdgeInsets.zero,
            constraints: const BoxConstraints(minWidth: 32, minHeight: 32),
          ),
        ],
      ),
    );
  }
}

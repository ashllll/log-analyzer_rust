/// 创建工作区对话框
/// 
/// 使用 StatefulWidget 管理表单状态
/// 业务逻辑提交给 AsyncNotifier

import 'package:flutter/material.dart';
import 'package:file_picker/file_picker.dart';

import '../../../domain/entities/workspace.dart';

/// 创建工作区对话框
class CreateWorkspaceDialog extends StatefulWidget {
  const CreateWorkspaceDialog({super.key});

  @override
  State<CreateWorkspaceDialog> createState() => _CreateWorkspaceDialogState();
}

class _CreateWorkspaceDialogState extends State<CreateWorkspaceDialog> {
  final _formKey = GlobalKey<FormState>();
  final _nameController = TextEditingController();
  final _pathController = TextEditingController();
  bool _isLoading = false;

  @override
  void dispose() {
    _nameController.dispose();
    _pathController.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    return AlertDialog(
      title: const Text('新建工作区'),
      content: Form(
        key: _formKey,
        child: Column(
          mainAxisSize: MainAxisSize.min,
          children: [
            // 名称输入
            TextFormField(
              controller: _nameController,
              decoration: const InputDecoration(
                labelText: '名称',
                hintText: '输入工作区名称',
                prefixIcon: Icon(Icons.edit),
              ),
              validator: (value) {
                if (value == null || value.trim().isEmpty) {
                  return '请输入工作区名称';
                }
                return null;
              },
              textInputAction: TextInputAction.next,
            ),
            const SizedBox(height: 16),
            
            // 路径输入
            TextFormField(
              controller: _pathController,
              decoration: InputDecoration(
                labelText: '路径',
                hintText: '选择日志文件夹',
                prefixIcon: const Icon(Icons.folder),
                suffixIcon: IconButton(
                  icon: const Icon(Icons.folder_open),
                  onPressed: _pickDirectory,
                ),
              ),
              validator: (value) {
                if (value == null || value.trim().isEmpty) {
                  return '请选择工作区路径';
                }
                return null;
              },
              readOnly: true,
              onTap: _pickDirectory,
            ),
          ],
        ),
      ),
      actions: [
        TextButton(
          onPressed: _isLoading ? null : () => Navigator.pop(context),
          child: const Text('取消'),
        ),
        FilledButton(
          onPressed: _isLoading ? null : _submit,
          child: _isLoading
              ? const SizedBox(
                  width: 20,
                  height: 20,
                  child: CircularProgressIndicator(
                    strokeWidth: 2,
                    color: Colors.white,
                  ),
                )
              : const Text('创建'),
        ),
      ],
    );
  }

  Future<void> _pickDirectory() async {
    final result = await FilePicker.platform.getDirectoryPath();
    if (result != null && mounted) {
      setState(() {
        _pathController.text = result;
      });
      
      // 自动生成名称（如果为空）
      if (_nameController.text.isEmpty) {
        final name = result.split('/').last.split('\\').last;
        _nameController.text = name;
      }
    }
  }

  void _submit() {
    if (_formKey.currentState?.validate() != true) {
      return;
    }

    final params = CreateWorkspaceParams(
      name: _nameController.text.trim(),
      path: _pathController.text.trim(),
    );

    Navigator.pop(context, params);
  }
}

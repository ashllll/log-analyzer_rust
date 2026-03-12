import 'dart:async';

import 'package:flutter/material.dart';

import '../../../../core/theme/app_theme.dart';
import '../../../../shared/services/bridge_service.dart';
import '../../../../shared/services/generated/ffi/types.dart';

/// 正则表达式输入框组件
///
/// 支持 300ms 防抖的实时语法验证，使用 FFI validateRegex API
/// 显示验证状态图标和错误/成功提示文本
///
/// 用法：
/// ```dart
/// RegexInputField(
///   controller: _regexController,
///   onValidChanged: (isValid) => setState(() => _regexValid = isValid),
///   focusNode: _regexFocusNode,
/// )
/// ```
class RegexInputField extends StatefulWidget {
  /// 文本控制器
  final TextEditingController controller;

  /// 验证状态变化回调
  final ValueChanged<bool>? onValidChanged;

  /// 焦点节点
  final FocusNode? focusNode;

  /// 占位提示文本
  final String? hintText;

  /// 是否大小写敏感
  final bool caseSensitive;

  /// 大小写敏感变化回调
  final ValueChanged<bool>? onCaseSensitiveChanged;

  const RegexInputField({
    super.key,
    required this.controller,
    this.onValidChanged,
    this.focusNode,
    this.hintText,
    this.caseSensitive = false,
    this.onCaseSensitiveChanged,
  });

  @override
  State<RegexInputField> createState() => _RegexInputFieldState();
}

class _RegexInputFieldState extends State<RegexInputField> {
  /// 验证结果
  RegexValidationResult? _validationResult;

  /// 防抖定时器
  Timer? _debounceTimer;

  /// 是否正在验证
  bool _isValidating = false;

  /// 上一次验证的文本（避免重复验证）
  String _lastValidatedText = '';

  @override
  void initState() {
    super.initState();
    widget.controller.addListener(_onTextChanged);
  }

  @override
  void dispose() {
    // 取消定时器，防止内存泄漏
    _debounceTimer?.cancel();
    widget.controller.removeListener(_onTextChanged);
    super.dispose();
  }

  /// 文本变化监听器
  void _onTextChanged() {
    final text = widget.controller.text;

    // 空文本时清除验证状态
    if (text.isEmpty) {
      setState(() {
        _validationResult = null;
        _lastValidatedText = '';
      });
      widget.onValidChanged?.call(false);
      _debounceTimer?.cancel();
      return;
    }

    // 如果文本没变化，不重复验证
    if (text == _lastValidatedText) {
      return;
    }

    // 取消之前的定时器
    _debounceTimer?.cancel();

    // 300ms 防抖
    _debounceTimer = Timer(const Duration(milliseconds: 300), () {
      _validateRegex(text);
    });
  }

  /// 验证正则表达式
  Future<void> _validateRegex(String pattern) async {
    if (pattern.isEmpty || pattern == _lastValidatedText) return;

    setState(() {
      _isValidating = true;
    });

    try {
      final bridge = BridgeService.instance;
      final result = await bridge.validateRegex(pattern);

      // 确保组件仍然挂载
      if (!mounted) return;

      setState(() {
        _validationResult = result;
        _lastValidatedText = pattern;
        _isValidating = false;
      });

      // 通知验证状态变化
      widget.onValidChanged?.call(result.valid);
    } catch (e) {
      if (!mounted) return;

      setState(() {
        _validationResult = RegexValidationResult(
          valid: false,
          errorMessage: '验证失败: $e',
        );
        _isValidating = false;
      });

      widget.onValidChanged?.call(false);
    }
  }

  @override
  Widget build(BuildContext context) {
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      mainAxisSize: MainAxisSize.min,
      children: [
        // 输入框
        TextField(
          controller: widget.controller,
          focusNode: widget.focusNode,
          decoration: _buildInputDecoration(),
          style: const TextStyle(
            fontSize: 14,
            fontFamily: 'FiraCode', // 等宽字体适合正则
          ),
          onSubmitted: (_) => _validateRegex(widget.controller.text),
        ),
        // 验证状态提示
        if (_validationResult != null || _isValidating) _buildValidationHint(),
      ],
    );
  }

  /// 构建输入框装饰
  InputDecoration _buildInputDecoration() {
    final hasText = widget.controller.text.isNotEmpty;
    final isValid = _validationResult?.valid ?? false;
    final hasError = _validationResult != null && !_validationResult!.valid;

    return InputDecoration(
      hintText: widget.hintText ?? '输入正则表达式，如: \\d+、[a-z]+、.*error.*',
      hintStyle: const TextStyle(color: AppColors.textMuted),
      // 前缀图标
      prefixIcon: const Icon(Icons.code, size: 20, color: AppColors.textMuted),
      // 后缀图标 - 验证状态
      suffixIcon: _buildSuffixIcon(hasText, isValid, hasError),
      // 错误边框
      errorText: hasError ? _validationResult!.errorMessage : null,
      // 帮助文本
      helperText: hasText && isValid ? '语法有效' : null,
      helperStyle: const TextStyle(color: AppColors.success, fontSize: 12),
      // 边框样式
      border: OutlineInputBorder(
        borderRadius: BorderRadius.circular(8),
        borderSide: const BorderSide(color: AppColors.border),
      ),
      enabledBorder: OutlineInputBorder(
        borderRadius: BorderRadius.circular(8),
        borderSide: BorderSide(
          color: hasError ? AppColors.error : AppColors.border,
        ),
      ),
      focusedBorder: OutlineInputBorder(
        borderRadius: BorderRadius.circular(8),
        borderSide: BorderSide(
          color: hasError ? AppColors.error : AppColors.primary,
          width: 2,
        ),
      ),
      // 内容内边距
      contentPadding: const EdgeInsets.symmetric(horizontal: 12, vertical: 12),
      // 填充色
      filled: true,
      fillColor: AppColors.bgInput,
    );
  }

  /// 构建后缀图标
  Widget? _buildSuffixIcon(bool hasText, bool isValid, bool hasError) {
    if (_isValidating) {
      // 正在验证
      return const SizedBox(
        width: 20,
        height: 20,
        child: Padding(
          padding: EdgeInsets.all(8),
          child: CircularProgressIndicator(
            strokeWidth: 2,
            valueColor: AlwaysStoppedAnimation<Color>(AppColors.primary),
          ),
        ),
      );
    }

    if (!hasText || _validationResult == null) {
      return null;
    }

    if (isValid) {
      // 有效的正则
      return const Icon(Icons.check_circle, color: AppColors.success, size: 20);
    }

    if (hasError) {
      // 无效的正则
      return IconButton(
        icon: const Icon(Icons.error, color: AppColors.error, size: 20),
        tooltip: _validationResult!.errorMessage,
        onPressed: () {
          // 显示完整错误信息
          ScaffoldMessenger.of(context).showSnackBar(
            SnackBar(
              content: Text(_validationResult!.errorMessage ?? '语法错误'),
              backgroundColor: AppColors.error,
              behavior: SnackBarBehavior.floating,
            ),
          );
        },
      );
    }

    return null;
  }

  /// 构建验证提示
  Widget _buildValidationHint() {
    if (_isValidating) {
      return const Padding(
        padding: EdgeInsets.only(top: 8),
        child: Row(
          children: [
            SizedBox(
              width: 12,
              height: 12,
              child: CircularProgressIndicator(
                strokeWidth: 1.5,
                valueColor: AlwaysStoppedAnimation<Color>(AppColors.textMuted),
              ),
            ),
            SizedBox(width: 8),
            Text(
              '正在验证语法...',
              style: TextStyle(color: AppColors.textMuted, fontSize: 12),
            ),
          ],
        ),
      );
    }

    final isValid = _validationResult?.valid ?? false;

    if (isValid) {
      return Padding(
        padding: const EdgeInsets.only(top: 8),
        child: Row(
          children: [
            const Icon(
              Icons.check_circle_outline,
              color: AppColors.success,
              size: 14,
            ),
            const SizedBox(width: 6),
            Text(
              '正则表达式语法有效',
              style: TextStyle(
                color: AppColors.success.withOpacity(0.9),
                fontSize: 12,
              ),
            ),
          ],
        ),
      );
    }

    // 错误信息已在 errorText 中显示，这里显示简短提示
    return Padding(
      padding: const EdgeInsets.only(top: 8),
      child: Row(
        children: [
          const Icon(Icons.error_outline, color: AppColors.error, size: 14),
          const SizedBox(width: 6),
          Expanded(
            child: Text(
              _validationResult?.errorMessage ?? '语法错误',
              style: const TextStyle(color: AppColors.error, fontSize: 12),
              maxLines: 1,
              overflow: TextOverflow.ellipsis,
            ),
          ),
        ],
      ),
    );
  }
}

import 'package:flutter/material.dart';
import 'package:log_analyzer_flutter/core/theme/app_theme.dart';

/// 自定义输入框组件
///
/// 对应 React 版本的 Input.tsx
/// 提供统一的输入框样式
///
/// 使用说明：
/// - 如果需要完全控制输入状态，传入 [controller] 参数
/// - 如果只需要简单的值绑定，传入 [value] 和 [onChanged] 参数
/// - 注意：不传 controller 时，组件内部会自动管理 controller 生命周期
class CustomInput extends StatefulWidget {
  final String? label;
  final String? hint;
  final String? value;
  final ValueChanged<String>? onChanged;
  final ValueChanged<String>? onSubmitted;
  final VoidCallback? onClear;
  final bool obscureText;
  final bool readOnly;
  final int? maxLines;
  final int? maxLength;
  final IconData? prefixIcon;
  final Widget? suffixIcon;
  final String? errorText;
  final TextInputType? keyboardType;
  final TextEditingController? controller;

  const CustomInput({
    super.key,
    this.label,
    this.hint,
    this.value,
    this.onChanged,
    this.onSubmitted,
    this.onClear,
    this.obscureText = false,
    this.readOnly = false,
    this.maxLines = 1,
    this.maxLength,
    this.prefixIcon,
    this.suffixIcon,
    this.errorText,
    this.keyboardType,
    this.controller,
  });

  @override
  State<CustomInput> createState() => _CustomInputState();
}

class _CustomInputState extends State<CustomInput> {
  // 内部 controller，当外部未提供时使用
  TextEditingController? _internalController;

  /// 获取有效的 controller
  TextEditingController get _effectiveController {
    // 如果外部提供了 controller，直接使用
    if (widget.controller != null) {
      return widget.controller!;
    }
    // 否则创建并缓存内部 controller
    _internalController ??= TextEditingController(text: widget.value ?? '');
    return _internalController!;
  }

  @override
  void didUpdateWidget(CustomInput oldWidget) {
    super.didUpdateWidget(oldWidget);
    // 当 value 变化且使用内部 controller 时，同步更新文本
    if (widget.controller == null &&
        widget.value != oldWidget.value &&
        widget.value != _effectiveController.text) {
      _effectiveController.text = widget.value ?? '';
    }
  }

  @override
  void dispose() {
    // 只释放内部创建的 controller
    _internalController?.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    return TextField(
      controller: _effectiveController,
      obscureText: widget.obscureText,
      readOnly: widget.readOnly,
      maxLines: widget.maxLines,
      maxLength: widget.maxLength,
      keyboardType: widget.keyboardType,
      onChanged: widget.onChanged,
      onSubmitted: widget.onSubmitted,
      decoration: InputDecoration(
        labelText: widget.label,
        hintText: widget.hint,
        prefixIcon: widget.prefixIcon != null ? Icon(widget.prefixIcon) : null,
        suffixIcon: _buildSuffixIcon(),
        errorText: widget.errorText,
        filled: true,
        fillColor: AppColors.bgInput,
        border: OutlineInputBorder(
          borderRadius: BorderRadius.circular(8),
          borderSide: const BorderSide(color: AppColors.border),
        ),
        enabledBorder: OutlineInputBorder(
          borderRadius: BorderRadius.circular(8),
          borderSide: const BorderSide(color: AppColors.border),
        ),
        focusedBorder: OutlineInputBorder(
          borderRadius: BorderRadius.circular(8),
          borderSide: const BorderSide(color: AppColors.primary),
        ),
        errorBorder: OutlineInputBorder(
          borderRadius: BorderRadius.circular(8),
          borderSide: const BorderSide(color: AppColors.error),
        ),
        contentPadding: const EdgeInsets.symmetric(
          horizontal: 12,
          vertical: 12,
        ),
      ),
      style: const TextStyle(
        color: AppColors.textPrimary,
        fontSize: 14,
      ),
    );
  }

  Widget? _buildSuffixIcon() {
    if (widget.suffixIcon != null) return widget.suffixIcon;

    // 使用 controller 的当前值来判断是否显示清除按钮
    if (widget.onClear != null && _effectiveController.text.isNotEmpty) {
      return IconButton(
        icon: const Icon(Icons.clear, size: 18),
        onPressed: widget.onClear,
      );
    }

    return null;
  }
}

/// 自定义搜索框组件
class CustomSearchInput extends StatefulWidget {
  final String? value;
  final ValueChanged<String>? onChanged;
  final ValueChanged<String>? onSubmitted;
  final String? hint;
  final VoidCallback? onClear;
  final bool debounceEnabled;
  final Duration debounceDuration;

  const CustomSearchInput({
    super.key,
    this.value,
    this.onChanged,
    this.onSubmitted,
    this.hint,
    this.onClear,
    this.debounceEnabled = true,
    this.debounceDuration = const Duration(milliseconds: 300),
  });

  @override
  State<CustomSearchInput> createState() => _CustomSearchInputState();
}

class _CustomSearchInputState extends State<CustomSearchInput> {
  late TextEditingController _controller;
  String _previousValue = '';

  @override
  void initState() {
    super.initState();
    _controller = TextEditingController(text: widget.value);
  }

  @override
  void dispose() {
    _controller.dispose();
    super.dispose();
  }

  @override
  void didUpdateWidget(CustomSearchInput oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (widget.value != _controller.text) {
      _controller.text = widget.value ?? '';
    }
  }

  void _onChanged(String value) {
    if (!widget.debounceEnabled) {
      widget.onChanged?.call(value);
      return;
    }

    _previousValue = value;

    // 防抖处理
    Future.delayed(widget.debounceDuration, () {
      if (_previousValue == _controller.text) {
        widget.onChanged?.call(_controller.text);
      }
    });
  }

  @override
  Widget build(BuildContext context) {
    return CustomInput(
      controller: _controller,
      hint: widget.hint ?? '搜索...',
      prefixIcon: Icons.search,
      onChanged: _onChanged,
      onSubmitted: widget.onSubmitted,
      onClear: widget.onClear ??
          () {
            _controller.clear();
            widget.onChanged?.call('');
          },
    );
  }
}

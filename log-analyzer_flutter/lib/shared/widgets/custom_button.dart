import 'package:flutter/material.dart';
import '../../core/theme/app_theme.dart';

/// 自定义按钮组件
///
/// 对应 React 版本的 Button.tsx
/// 支持多种样式变体和加载状态
/// 支持无障碍访问
class CustomButton extends StatelessWidget {
  final String text;
  final VoidCallback? onPressed;
  final ButtonVariant variant;
  final bool isLoading;
  final bool isFullWidth;
  final Widget? icon;
  final ButtonSize size;
  /// 无障碍标签
  final String? semanticLabel;

  const CustomButton({
    super.key,
    required this.text,
    this.onPressed,
    this.variant = ButtonVariant.primary,
    this.isLoading = false,
    this.isFullWidth = false,
    this.icon,
    this.size = ButtonSize.medium,
    this.semanticLabel,
  });

  @override
  Widget build(BuildContext context) {
    final isEnabled = onPressed != null && !isLoading;

    Widget buttonChild = Row(
      mainAxisSize: isFullWidth ? MainAxisSize.max : MainAxisSize.min,
      mainAxisAlignment: MainAxisAlignment.center,
      children: [
        if (isLoading)
          const SizedBox(
            width: 16,
            height: 16,
            child: CircularProgressIndicator(
              strokeWidth: 2,
              color: Colors.white,
            ),
          )
        else if (icon != null)
          icon!
        else
          const SizedBox.shrink(),
        if (isLoading || icon != null) const SizedBox(width: 8),
        Text(text),
      ],
    );

    // 构建无障碍标签
    final String label = semanticLabel ?? (isLoading ? '$text 加载中' : text);

    Widget buildButton(Widget child) {
      return Semantics(
        button: true,
        label: label,
        enabled: isEnabled,
        child: child,
      );
    }

    switch (variant) {
      case ButtonVariant.primary:
        return buildButton(
          SizedBox(
            width: isFullWidth ? double.infinity : null,
            child: ElevatedButton(
              onPressed: isEnabled ? onPressed : null,
              style: ElevatedButton.styleFrom(
                backgroundColor: isEnabled ? AppColors.primary : AppColors.bgHover,
                foregroundColor: Colors.white,
                disabledBackgroundColor: AppColors.bgHover,
                padding: _getPadding(),
                shape: RoundedRectangleBorder(
                  borderRadius: BorderRadius.circular(8),
                ),
              ),
              child: buttonChild,
            ),
          ),
        );

      case ButtonVariant.secondary:
        return buildButton(
          SizedBox(
            width: isFullWidth ? double.infinity : null,
            child: OutlinedButton(
              onPressed: isEnabled ? onPressed : null,
              style: OutlinedButton.styleFrom(
                foregroundColor: AppColors.textPrimary,
                side: const BorderSide(color: AppColors.border),
                padding: _getPadding(),
                shape: RoundedRectangleBorder(
                  borderRadius: BorderRadius.circular(8),
                ),
              ),
              child: buttonChild,
            ),
          ),
        );

      case ButtonVariant.ghost:
        return buildButton(
          SizedBox(
            width: isFullWidth ? double.infinity : null,
            child: TextButton(
              onPressed: isEnabled ? onPressed : null,
              style: TextButton.styleFrom(
                foregroundColor: AppColors.textSecondary,
                padding: _getPadding(),
              ),
              child: buttonChild,
            ),
          ),
        );

      case ButtonVariant.danger:
        return buildButton(
          SizedBox(
            width: isFullWidth ? double.infinity : null,
            child: ElevatedButton(
              onPressed: isEnabled ? onPressed : null,
              style: ElevatedButton.styleFrom(
                backgroundColor: isEnabled ? AppColors.error : AppColors.bgHover,
                foregroundColor: Colors.white,
                disabledBackgroundColor: AppColors.bgHover,
                padding: _getPadding(),
                shape: RoundedRectangleBorder(
                  borderRadius: BorderRadius.circular(8),
                ),
              ),
              child: buttonChild,
            ),
          ),
        );

      case ButtonVariant.active:
        return buildButton(
          SizedBox(
            width: isFullWidth ? double.infinity : null,
            child: ElevatedButton(
              onPressed: isEnabled ? onPressed : null,
              style: ElevatedButton.styleFrom(
                backgroundColor: AppColors.bgHover,
                foregroundColor: AppColors.textPrimary,
                padding: _getPadding(),
                shape: RoundedRectangleBorder(
                  borderRadius: BorderRadius.circular(8),
                  side: const BorderSide(color: AppColors.borderLight),
                ),
              ),
              child: buttonChild,
            ),
          ),
        );
    }
  }

  EdgeInsetsGeometry _getPadding() {
    switch (size) {
      case ButtonSize.small:
        return const EdgeInsets.symmetric(horizontal: 12, vertical: 8);
      case ButtonSize.medium:
        return const EdgeInsets.symmetric(horizontal: 16, vertical: 12);
      case ButtonSize.large:
        return const EdgeInsets.symmetric(horizontal: 20, vertical: 16);
    }
  }
}

/// 按钮样式变体
enum ButtonVariant {
  primary,
  secondary,
  ghost,
  danger,
  active,
}

/// 按钮尺寸
enum ButtonSize {
  small,
  medium,
  large,
}

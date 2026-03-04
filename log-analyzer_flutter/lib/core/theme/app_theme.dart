import 'package:flutter/material.dart';

/// 主题配置
///
/// 对应 React 版本的 Tailwind 配置 (tailwind.config.js)
/// 颜色方案保持一致，使用 Material 3 设计语言

/// 亮色主题
///
/// 颜色映射关系：
/// - bg-main: Color(0xFFFAFAFA) -> scaffoldBackgroundColor
/// - bg-card: Color(0xFFFFFFFF) -> cardColor / surfaceColor
/// - primary: Color(0xFF2563EB) -> primary
/// - border: Color(0xFFE4E4E7) -> dividerColor
ThemeData lightTheme() {
  const colorScheme = ColorScheme.light(
    primary: Color(0xFF2563EB),      // primary (blue-600)
    secondary: Color(0xFF10B981),    // green-500
    error: Color(0xFFEF4444),        // red-500
    surface: Color(0xFFFFFFFF),      // bg-card (white)
    onPrimary: Colors.white,
    onSecondary: Colors.white,
    onError: Colors.white,
    onSurface: Color(0xFF18181B),    // text-primary (zinc-900)
  );

  return ThemeData.light().copyWith(
    colorScheme: colorScheme,

    // 脚手架背景色 - bg-main (zinc-50)
    scaffoldBackgroundColor: const Color(0xFFFAFAFA),

    // 卡片颜色 - bg-card
    cardColor: const Color(0xFFFFFFFF),

    // 应用栏主题
    appBarTheme: const AppBarTheme(
      backgroundColor: Color(0xFFFFFFFF),
      elevation: 0,
      centerTitle: false,
      titleTextStyle: TextStyle(
        color: Color(0xFF18181B),    // text-primary
        fontSize: 18,
        fontWeight: FontWeight.w600,
      ),
      iconTheme: IconThemeData(color: Color(0xFF18181B)),
    ),

    // 导航栏主题
    navigationBarTheme: NavigationBarThemeData(
      backgroundColor: const Color(0xFFFFFFFF),
      indicatorColor: const Color(0xFFE4E4E7),
      labelTextStyle: WidgetStatePropertyAll(
        TextStyle(color: Colors.grey[700]),
      ),
    ),

    // 输入框主题
    inputDecorationTheme: InputDecorationTheme(
      filled: true,
      fillColor: const Color(0xFFF4F4F5),  // bg-input
      border: OutlineInputBorder(
        borderRadius: BorderRadius.circular(8),
        borderSide: const BorderSide(color: Color(0xFFE4E4E7)),
      ),
      enabledBorder: OutlineInputBorder(
        borderRadius: BorderRadius.circular(8),
        borderSide: const BorderSide(color: Color(0xFFE4E4E7)),
      ),
      focusedBorder: OutlineInputBorder(
        borderRadius: BorderRadius.circular(8),
        borderSide: const BorderSide(color: Color(0xFF2563EB)),
      ),
      contentPadding: const EdgeInsets.symmetric(horizontal: 12, vertical: 8),
    ),

    // 按钮主题
    elevatedButtonTheme: ElevatedButtonThemeData(
      style: ElevatedButton.styleFrom(
        backgroundColor: const Color(0xFF2563EB),
        foregroundColor: Colors.white,
        padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 12),
        shape: RoundedRectangleBorder(
          borderRadius: BorderRadius.circular(8),
        ),
      ),
    ),

    outlinedButtonTheme: OutlinedButtonThemeData(
      style: OutlinedButton.styleFrom(
        foregroundColor: const Color(0xFF18181B),
        side: const BorderSide(color: Color(0xFFE4E4E7)),
        padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 12),
        shape: RoundedRectangleBorder(
          borderRadius: BorderRadius.circular(8),
        ),
      ),
    ),

    textButtonTheme: TextButtonThemeData(
      style: TextButton.styleFrom(
        foregroundColor: const Color(0xFF2563EB),
        padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 8),
      ),
    ),

    // 分割线颜色 - border
    dividerColor: const Color(0xFFE4E4E7),

    // 文本主题
    textTheme: const TextTheme(
      displayLarge: TextStyle(
        color: Color(0xFF18181B),
        fontSize: 32,
        fontWeight: FontWeight.bold,
      ),
      displayMedium: TextStyle(
        color: Color(0xFF18181B),
        fontSize: 24,
        fontWeight: FontWeight.w600,
      ),
      bodyLarge: TextStyle(
        color: Color(0xFF52525B),
        fontSize: 16,
      ),
      bodyMedium: TextStyle(
        color: Color(0xFF52525B),
        fontSize: 14,
      ),
      labelLarge: TextStyle(
        color: Color(0xFF18181B),
        fontSize: 14,
        fontWeight: FontWeight.w500,
      ),
    ),

    // Icon 主题
    iconTheme: const IconThemeData(
      color: Color(0xFF52525B),
      size: 20,
    ),

    // 对话框主题
    dialogTheme: const DialogThemeData(
      backgroundColor: Color(0xFFFFFFFF),
      shape: RoundedRectangleBorder(
        borderRadius: BorderRadius.all(Radius.circular(12)),
      ),
      surfaceTintColor: Colors.transparent,
    ),

    // SnackBar 主题
    snackBarTheme: SnackBarThemeData(
      backgroundColor: const Color(0xFF27272A),
      contentTextStyle: const TextStyle(color: Color(0xFFFAFAFA)),
      shape: RoundedRectangleBorder(
        borderRadius: BorderRadius.circular(8),
      ),
      behavior: SnackBarBehavior.floating,
    ),

    // Tooltip 主题
    tooltipTheme: TooltipThemeData(
      decoration: BoxDecoration(
        color: const Color(0xFF18181B),
        borderRadius: BorderRadius.circular(6),
      ),
      textStyle: const TextStyle(color: Color(0xFFFAFAFA)),
    ),

    // Card 主题
    cardTheme: CardThemeData(
      color: const Color(0xFFFFFFFF),
      elevation: 0,
      shape: RoundedRectangleBorder(
        borderRadius: BorderRadius.circular(8),
        side: const BorderSide(color: Color(0xFFE4E4E7)),
      ),
    ),
  );
}

/// 深色主题（默认主题）
///
/// 颜色映射关系：
/// - bg-main: Color(0xFF09090B) -> scaffoldBackgroundColor
/// - bg-card: Color(0xFF27272A) -> cardColor / surfaceColor
/// - primary: Color(0xFF2563EB) -> primary
/// - border: Color(0xFF3F3F46) -> dividerColor
ThemeData darkTheme() {
  const colorScheme = ColorScheme.dark(
    primary: Color(0xFF2563EB),      // primary (blue-600)
    secondary: Color(0xFF10B981),    // green-500
    error: Color(0xFFEF4444),        // red-500
    surface: Color(0xFF27272A),      // bg-card (zinc-800)
    onPrimary: Colors.white,
    onSecondary: Colors.white,
    onError: Colors.white,
    onSurface: Color(0xFFA1A1AA),    // text-muted (zinc-400)
  );

  return ThemeData.dark().copyWith(
    colorScheme: colorScheme,

    // 脚手架背景色 - bg-main (zinc-950)
    scaffoldBackgroundColor: const Color(0xFF09090B),

    // 卡片颜色 - bg-card
    cardColor: const Color(0xFF27272A),

    // 应用栏主题
    appBarTheme: const AppBarTheme(
      backgroundColor: Color(0xFF09090B),
      elevation: 0,
      centerTitle: false,
      titleTextStyle: TextStyle(
        color: Color(0xFFFAFAFA),    // text-primary
        fontSize: 18,
        fontWeight: FontWeight.w600,
      ),
    ),

    // 导航栏主题
    navigationBarTheme: const NavigationBarThemeData(
      backgroundColor: Color(0xFF27272A),
      indicatorColor: Color(0xFF3F3F46),
      labelTextStyle: WidgetStatePropertyAll(
        TextStyle(color: Color(0xFFA1A1AA)),
      ),
    ),

    // 输入框主题
    inputDecorationTheme: InputDecorationTheme(
      filled: true,
      fillColor: const Color(0xFF18181B),  // bg-input
      border: OutlineInputBorder(
        borderRadius: BorderRadius.circular(8),
        borderSide: const BorderSide(color: Color(0xFF3F3F46)),
      ),
      enabledBorder: OutlineInputBorder(
        borderRadius: BorderRadius.circular(8),
        borderSide: const BorderSide(color: Color(0xFF3F3F46)),
      ),
      focusedBorder: OutlineInputBorder(
        borderRadius: BorderRadius.circular(8),
        borderSide: const BorderSide(color: Color(0xFF2563EB)),
      ),
      contentPadding: const EdgeInsets.symmetric(horizontal: 12, vertical: 8),
    ),

    // 按钮主题
    elevatedButtonTheme: ElevatedButtonThemeData(
      style: ElevatedButton.styleFrom(
        backgroundColor: const Color(0xFF2563EB),
        foregroundColor: Colors.white,
        padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 12),
        shape: RoundedRectangleBorder(
          borderRadius: BorderRadius.circular(8),
        ),
      ),
    ),

    outlinedButtonTheme: OutlinedButtonThemeData(
      style: OutlinedButton.styleFrom(
        foregroundColor: const Color(0xFFFAFAFA),
        side: const BorderSide(color: Color(0xFF3F3F46)),
        padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 12),
        shape: RoundedRectangleBorder(
          borderRadius: BorderRadius.circular(8),
        ),
      ),
    ),

    textButtonTheme: TextButtonThemeData(
      style: TextButton.styleFrom(
        foregroundColor: const Color(0xFF2563EB),
        padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 8),
      ),
    ),

    // 分割线颜色 - border
    dividerColor: const Color(0xFF3F3F46),

    // 文本主题
    textTheme: const TextTheme(
      displayLarge: TextStyle(
        color: Color(0xFFFAFAFA),
        fontSize: 32,
        fontWeight: FontWeight.bold,
      ),
      displayMedium: TextStyle(
        color: Color(0xFFFAFAFA),
        fontSize: 24,
        fontWeight: FontWeight.w600,
      ),
      bodyLarge: TextStyle(
        color: Color(0xFFA1A1AA),
        fontSize: 16,
      ),
      bodyMedium: TextStyle(
        color: Color(0xFFA1A1AA),
        fontSize: 14,
      ),
      labelLarge: TextStyle(
        color: Color(0xFFFAFAFA),
        fontSize: 14,
        fontWeight: FontWeight.w500,
      ),
    ),

    // Icon 主题
    iconTheme: const IconThemeData(
      color: Color(0xFFA1A1AA),
      size: 20,
    ),

    // 对话框主题
    dialogTheme: const DialogThemeData(
      backgroundColor: Color(0xFF27272A),
      shape: RoundedRectangleBorder(
        borderRadius: BorderRadius.all(Radius.circular(12)),
      ),
    ),

    // SnackBar 主题
    snackBarTheme: SnackBarThemeData(
      backgroundColor: const Color(0xFF27272A),
      contentTextStyle: const TextStyle(color: Color(0xFFFAFAFA)),
      shape: RoundedRectangleBorder(
        borderRadius: BorderRadius.circular(8),
      ),
      behavior: SnackBarBehavior.floating,
    ),

    // Tooltip 主题
    tooltipTheme: const TooltipThemeData(
      decoration: BoxDecoration(
        color: Color(0xFF18181B),
        borderRadius: BorderRadius.all(Radius.circular(6)),
      ),
      textStyle: TextStyle(color: Color(0xFFFAFAFA)),
    ),

    // Card 主题
    cardTheme: const CardThemeData(
      color: Color(0xFF27272A),
      elevation: 0,
      shape: RoundedRectangleBorder(
        borderRadius: BorderRadius.all(Radius.circular(8)),
        side: BorderSide(color: Color(0xFF3F3F46)),
      ),
    ),
  );
}

/// 颜色常量（与 Tailwind 配色一致）
class AppColors {
  // 主色调
  static const primary = Color(0xFF2563EB);
  static const primaryHover = Color(0xFF1D4ED8);
  static const primaryLight = Color(0xFF3B82F6);

  // 成功/绿色
  static const success = Color(0xFF10B981);
  static const successHover = Color(0xFF059669);
  static const successLight = Color(0xFF34D399);

  // 错误/红色
  static const error = Color(0xFFEF4444);
  static const errorHover = Color(0xFFDC2626);
  static const errorLight = Color(0xFFF87171);

  // 警告/橙色
  static const warning = Color(0xFFF59E0B);
  static const warningHover = Color(0xFFD97706);
  static const warningLight = Color(0xFFFBBF24);

  // 背景色
  static const bgMain = Color(0xFF09090B);
  static const bgCard = Color(0xFF27272A);
  static const bgInput = Color(0xFF18181B);
  static const bgHover = Color(0xFF3F3F46);

  // 文本色
  static const textPrimary = Color(0xFFFAFAFA);
  static const textSecondary = Color(0xFFA1A1AA);
  static const textMuted = Color(0xFF71717A);

  // 边框色
  static const border = Color(0xFF3F3F46);
  static const borderLight = Color(0xFF52525B);
  static const borderDark = Color(0xFF27272A);

  // 关键词颜色（与 React 版本一致）
  static const keywordBlue = Color(0xFF3B82F6);
  static const keywordGreen = Color(0xFF10B981);
  static const keywordRed = Color(0xFFEF4444);
  static const keywordOrange = Color(0xFFF59E0B);
  static const keywordPurple = Color(0xFFA855F7);

  /// 根据颜色键获取对应的颜色值
  static Color fromColorKey(String colorKey) {
    switch (colorKey) {
      case 'blue':
        return keywordBlue;
      case 'green':
        return keywordGreen;
      case 'red':
        return keywordRed;
      case 'orange':
        return keywordOrange;
      case 'purple':
        return keywordPurple;
      default:
        return keywordBlue;
    }
  }
}

/// 搜索模式枚举
///
/// 定义支持的三种搜索模式：
/// - normal: 普通文本搜索
/// - regex: 正则表达式搜索
/// - combined: 组合搜索（正则 + 关键词）
enum SearchMode {
  /// 普通文本搜索
  /// 使用简单的字符串匹配
  normal,

  /// 正则表达式搜索
  /// 支持完整的正则语法
  regex,

  /// 组合搜索
  /// 同时使用正则和关键词搜索
  combined;

  /// 获取显示名称
  String get displayName {
    switch (this) {
      case SearchMode.normal:
        return '普通';
      case SearchMode.regex:
        return '正则';
      case SearchMode.combined:
        return '组合';
    }
  }

  /// 获取描述文本
  String get description {
    switch (this) {
      case SearchMode.normal:
        return '简单文本搜索，匹配任意字符';
      case SearchMode.regex:
        return '正则表达式搜索，支持复杂模式匹配';
      case SearchMode.combined:
        return '组合搜索，同时使用关键词和正则';
    }
  }

  /// 获取图标
  ///
  /// 用于在 UI 中显示模式图标
  String get icon {
    switch (this) {
      case SearchMode.normal:
        return 'search';
      case SearchMode.regex:
        return 'code';
      case SearchMode.combined:
        return 'manage_search';
    }
  }
}

/// 日志条目实体
/// 
/// 表示单条日志记录

import 'package:dart_mappable/dart_mappable.dart';
import 'package:equatable/equatable.dart';

part 'log_entry.mapper.dart';

/// 日志级别
enum LogLevel {
  /// 调试
  debug,
  /// 信息
  info,
  /// 警告
  warning,
  /// 错误
  error,
  /// 致命
  fatal,
  /// 未知
  unknown,
}

/// 日志条目实体
@MappableClass()
class LogEntry with LogEntryMappable, EquatableMixin {
  /// 唯一标识
  final String id;
  
  /// 原始日志行号
  final int lineNumber;
  
  /// 日志内容
  final String content;
  
  /// 日志级别
  final LogLevel level;
  
  /// 时间戳
  final DateTime? timestamp;
  
  /// 来源文件路径
  final String? sourceFile;
  
  /// 匹配的关键词
  final List<String>? matchedKeywords;
  
  /// 是否被选中
  final bool isSelected;
  
  /// 额外的元数据
  final Map<String, dynamic>? metadata;

  const LogEntry({
    required this.id,
    required this.lineNumber,
    required this.content,
    this.level = LogLevel.unknown,
    this.timestamp,
    this.sourceFile,
    this.matchedKeywords,
    this.isSelected = false,
    this.metadata,
  });

  /// 空日志条目
  static const empty = LogEntry(
    id: '',
    lineNumber: 0,
    content: '',
  );

  /// 日志级别显示名称
  String get levelDisplayName {
    switch (level) {
      case LogLevel.debug:
        return 'DEBUG';
      case LogLevel.info:
        return 'INFO';
      case LogLevel.warning:
        return 'WARN';
      case LogLevel.error:
        return 'ERROR';
      case LogLevel.fatal:
        return 'FATAL';
      case LogLevel.unknown:
        return 'UNKNOWN';
    }
  }

  /// 日志级别颜色（Material 颜色值）
  int get levelColor {
    switch (level) {
      case LogLevel.debug:
        return 0xFF2196F3; // Blue
      case LogLevel.info:
        return 0xFF4CAF50; // Green
      case LogLevel.warning:
        return 0xFFFF9800; // Orange
      case LogLevel.error:
        return 0xFFF44336; // Red
      case LogLevel.fatal:
        return 0xFF9C27B0; // Purple
      case LogLevel.unknown:
        return 0xFF9E9E9E; // Grey
    }
  }

  /// 截断的内容（用于预览）
  String get previewContent {
    if (content.length <= 100) return content;
    return '${content.substring(0, 100)}...';
  }

  /// 是否有匹配关键词
  bool get hasMatchedKeywords => 
      matchedKeywords != null && matchedKeywords!.isNotEmpty;

  /// 高亮显示内容
  String get highlightedContent {
    // 这里可以返回带高亮标记的内容
    // 实际高亮在 UI 层处理
    return content;
  }

  @override
  List<Object?> get props => [
    id,
    lineNumber,
    content,
    level,
    timestamp,
    sourceFile,
    isSelected,
  ];
}

/// 搜索结果
@MappableClass()
class SearchResult with SearchResultMappable, EquatableMixin {
  /// 搜索 ID
  final String searchId;
  
  /// 日志条目列表
  final List<LogEntry> entries;
  
  /// 总匹配数
  final int totalMatches;
  
  /// 扫描的文件数
  final int scannedFiles;
  
  /// 搜索耗时（毫秒）
  final int durationMs;
  
  /// 是否完成
  final bool isComplete;
  
  /// 匹配的文件列表
  final List<String> matchedFiles;

  const SearchResult({
    required this.searchId,
    this.entries = const [],
    this.totalMatches = 0,
    this.scannedFiles = 0,
    this.durationMs = 0,
    this.isComplete = false,
    this.matchedFiles = const [],
  });

  static const empty = SearchResult(searchId: '');

  /// 是否为空结果
  bool get isEmpty => entries.isEmpty;

  /// 是否有更多结果
  bool get hasMore => entries.length < totalMatches;

  /// 添加更多条目
  SearchResult addEntries(List<LogEntry> newEntries) {
    return copyWith(
      entries: [...entries, ...newEntries],
    );
  }

  @override
  List<Object?> get props => [
    searchId,
    entries,
    totalMatches,
    scannedFiles,
    durationMs,
    isComplete,
  ];
}

/// 搜索参数
@MappableClass()
class SearchParams with SearchParamsMappable {
  /// 查询字符串
  final String query;
  
  /// 工作区 ID
  final String? workspaceId;
  
  /// 最大结果数
  final int maxResults;
  
  /// 日志级别过滤
  final List<LogLevel>? levels;
  
  /// 时间范围开始
  final DateTime? timeRangeStart;
  
  /// 时间范围结束
  final DateTime? timeRangeEnd;
  
  /// 文件模式过滤
  final String? filePattern;
  
  /// 是否正则搜索
  final bool isRegex;
  
  /// 是否大小写敏感
  final bool caseSensitive;
  
  /// 关键词列表（组合搜索）
  final List<String>? keywords;
  
  /// 全局操作符（AND/OR/NOT）
  final String? globalOperator;

  const SearchParams({
    required this.query,
    this.workspaceId,
    this.maxResults = 10000,
    this.levels,
    this.timeRangeStart,
    this.timeRangeEnd,
    this.filePattern,
    this.isRegex = false,
    this.caseSensitive = false,
    this.keywords,
    this.globalOperator = 'AND',
  });

  /// 验证参数
  String? validate() {
    if (query.trim().isEmpty && (keywords == null || keywords!.isEmpty)) {
      return '搜索内容不能为空';
    }
    if (isRegex) {
      // 验证正则表达式
      try {
        RegExp(query);
      } catch (e) {
        return '无效的正则表达式: $e';
      }
    }
    return null;
  }

  bool get isValid => validate() == null;

  /// 是否是组合搜索
  bool get isCombinedSearch => keywords != null && keywords!.length > 1;
}

/// 日志级别统计
@MappableClass()
class LogLevelStats with LogLevelStatsMappable {
  /// 各级别数量
  final Map<LogLevel, int> counts;
  
  /// 总数
  final int total;

  const LogLevelStats({
    this.counts = const {},
    this.total = 0,
  });

  factory LogLevelStats.fromMap(Map<String, int> map) {
    final counts = <LogLevel, int>{};
    for (final entry in map.entries) {
      final level = _parseLevel(entry.key);
      counts[level] = entry.value;
    }
    return LogLevelStats(
      counts: counts,
      total: counts.values.fold(0, (a, b) => a + b),
    );
  }

  static LogLevel _parseLevel(String level) {
    switch (level.toUpperCase()) {
      case 'DEBUG':
        return LogLevel.debug;
      case 'INFO':
        return LogLevel.info;
      case 'WARN':
      case 'WARNING':
        return LogLevel.warning;
      case 'ERROR':
        return LogLevel.error;
      case 'FATAL':
        return LogLevel.fatal;
      default:
        return LogLevel.unknown;
    }
  }

  /// 获取指定级别的百分比
  double getPercentage(LogLevel level) {
    if (total == 0) return 0;
    final count = counts[level] ?? 0;
    return (count / total) * 100;
  }
}

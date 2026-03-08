import 'dart:convert';

/// 搜索条件模型
///
/// 对应 Rust 后端的 SearchTermData 结构体
class SearchTerm {
  /// 条件唯一标识
  final String id;

  /// 搜索值/关键词
  final String value;

  /// 该条件的操作符（AND, OR, NOT）
  final String operator;

  /// 是否为正则表达式
  final bool isRegex;

  /// 优先级（数字越小优先级越高）
  final int priority;

  /// 是否启用该条件
  final bool enabled;

  /// 是否大小写敏感
  final bool caseSensitive;

  const SearchTerm({
    required this.id,
    required this.value,
    required this.operator,
    required this.isRegex,
    required this.priority,
    required this.enabled,
    required this.caseSensitive,
  });

  /// 从 JSON 创建
  factory SearchTerm.fromJson(Map<String, dynamic> json) {
    return SearchTerm(
      id: json['id'] as String,
      value: json['value'] as String,
      operator: json['operator'] as String,
      isRegex: json['is_regex'] as bool? ?? false,
      priority: json['priority'] as int? ?? 0,
      enabled: json['enabled'] as bool? ?? true,
      caseSensitive: json['case_sensitive'] as bool? ?? false,
    );
  }

  /// 转换为 JSON
  Map<String, dynamic> toJson() {
    return {
      'id': id,
      'value': value,
      'operator': operator,
      'is_regex': isRegex,
      'priority': priority,
      'enabled': enabled,
      'case_sensitive': caseSensitive,
    };
  }

  @override
  String toString() =>
      'SearchTerm(id: $id, value: $value, operator: $operator, isRegex: $isRegex)';
}

/// 时间范围模型
class TimeRange {
  /// 起始时间（ISO 8601 格式）
  final String? start;

  /// 结束时间（ISO 8601 格式）
  final String? end;

  const TimeRange({
    this.start,
    this.end,
  });

  /// 从 JSON 创建
  factory TimeRange.fromJson(Map<String, dynamic>? json) {
    if (json == null) return const TimeRange();
    return TimeRange(
      start: json['start'] as String?,
      end: json['end'] as String?,
    );
  }

  /// 转换为 JSON
  Map<String, dynamic>? toJson() {
    if (start == null && end == null) return null;
    return {
      'start': start,
      'end': end,
    };
  }

  @override
  String toString() => 'TimeRange(start: $start, end: $end)';
}

/// 保存的过滤器模型
///
/// 对应 Rust 后端的 SavedFilterData 结构体
/// 用于 Flutter 端过滤器展示和管理
class SavedFilter {
  /// 过滤器唯一标识
  final String id;

  /// 过滤器名称
  final String name;

  /// 过滤器描述（可选）
  final String? description;

  /// 工作区ID
  final String workspaceId;

  /// 搜索条件列表
  final List<SearchTerm> terms;

  /// 全局操作符 (AND, OR, NOT)
  final String globalOperator;

  /// 时间范围
  final TimeRange? timeRange;

  /// 日志级别列表
  final List<String> levels;

  /// 文件模式
  final String? filePattern;

  /// 是否为默认过滤器
  final bool isDefault;

  /// 排序权重
  final int sortOrder;

  /// 使用次数
  final int usageCount;

  /// 创建时间（ISO 8601 格式）
  final String createdAt;

  /// 最后使用时间（ISO 8601 格式，可选）
  final String? lastUsedAt;

  const SavedFilter({
    required this.id,
    required this.name,
    this.description,
    required this.workspaceId,
    required this.terms,
    required this.globalOperator,
    this.timeRange,
    required this.levels,
    this.filePattern,
    required this.isDefault,
    required this.sortOrder,
    required this.usageCount,
    required this.createdAt,
    this.lastUsedAt,
  });

  /// 从 JSON 创建（用于 FFI 数据转换）
  factory SavedFilter.fromJson(Map<String, dynamic> json) {
    return SavedFilter(
      id: json['id'] as String,
      name: json['name'] as String,
      description: json['description'] as String?,
      workspaceId: json['workspace_id'] as String,
      terms: (json['terms'] as List?)
              ?.map((t) => SearchTerm.fromJson(t as Map<String, dynamic>))
              .toList() ??
          [],
      globalOperator: json['global_operator'] as String? ?? 'AND',
      timeRange: json['time_range'] != null
          ? TimeRange.fromJson(json['time_range'] as Map<String, dynamic>?)
          : null,
      levels: (json['levels'] as List?)?.cast<String>() ?? [],
      filePattern: json['file_pattern'] as String?,
      isDefault: json['is_default'] as bool? ?? false,
      sortOrder: json['sort_order'] as int? ?? 0,
      usageCount: json['usage_count'] as int? ?? 0,
      createdAt: json['created_at'] as String,
      lastUsedAt: json['last_used_at'] as String?,
    );
  }

  /// 转换为 JSON
  Map<String, dynamic> toJson() {
    return {
      'id': id,
      'name': name,
      'description': description,
      'workspace_id': workspaceId,
      'terms': terms.map((t) => t.toJson()).toList(),
      'global_operator': globalOperator,
      'time_range': timeRange?.toJson(),
      'levels': levels,
      'file_pattern': filePattern,
      'is_default': isDefault,
      'sort_order': sortOrder,
      'usage_count': usageCount,
      'created_at': createdAt,
      'last_used_at': lastUsedAt,
    };
  }

  /// 从 FFI Map 创建（运行时转换）
  ///
  /// Flutter FFI 生成的类型转换为本地模型
  factory SavedFilter.fromFfiMap(Map<String, dynamic> data) {
    // 解析 terms JSON
    List<SearchTerm> terms = [];
    final termsJson = data['terms_json'];
    if (termsJson != null && termsJson is String && termsJson.isNotEmpty) {
      try {
        final termsList = jsonDecode(termsJson) as List;
        terms = termsList
            .map((json) => SearchTerm.fromJson(json as Map<String, dynamic>))
            .toList();
      } catch (e) {
        // 解析失败时返回空列表
        terms = [];
      }
    }

    // 解析 levels JSON
    List<String> levels = [];
    final levelsJson = data['levels_json'];
    if (levelsJson != null && levelsJson is String && levelsJson.isNotEmpty) {
      try {
        levels = (jsonDecode(levelsJson) as List).cast<String>();
      } catch (e) {
        levels = [];
      }
    }

    // 构建时间范围
    TimeRange? timeRange;
    final timeRangeStart = data['time_range_start'];
    final timeRangeEnd = data['time_range_end'];
    if (timeRangeStart != null || timeRangeEnd != null) {
      timeRange = TimeRange(
        start: timeRangeStart as String?,
        end: timeRangeEnd as String?,
      );
    }

    return SavedFilter(
      id: data['id'] as String,
      name: data['name'] as String,
      description: data['description'] as String?,
      workspaceId: data['workspace_id'] as String,
      terms: terms,
      globalOperator: data['global_operator'] as String? ?? 'AND',
      timeRange: timeRange,
      levels: levels,
      filePattern: data['file_pattern'] as String?,
      isDefault: data['is_default'] as bool? ?? false,
      sortOrder: data['sort_order'] as int? ?? 0,
      usageCount: data['usage_count'] as int? ?? 0,
      createdAt: data['created_at'] as String,
      lastUsedAt: data['last_used_at'] as String?,
    );
  }

  /// 转换为 FFI Map（用于传递给 Rust 后端）
  ///
  /// 返回适合 SavedFilterInput 的 Map 结构
  Map<String, dynamic> toFfiMap() {
    return {
      'name': name,
      'description': description,
      'workspace_id': workspaceId,
      'terms_json': jsonEncode(terms.map((t) => t.toJson()).toList()),
      'global_operator': globalOperator,
      'time_range_start': timeRange?.start,
      'time_range_end': timeRange?.end,
      'levels_json': levels.isNotEmpty ? jsonEncode(levels) : null,
      'file_pattern': filePattern,
      'is_default': isDefault,
      'sort_order': sortOrder,
    };
  }

  /// 创建副本（用于更新）
  SavedFilter copyWith({
    String? id,
    String? name,
    String? description,
    String? workspaceId,
    List<SearchTerm>? terms,
    String? globalOperator,
    TimeRange? timeRange,
    List<String>? levels,
    String? filePattern,
    bool? isDefault,
    int? sortOrder,
    int? usageCount,
    String? createdAt,
    String? lastUsedAt,
  }) {
    return SavedFilter(
      id: id ?? this.id,
      name: name ?? this.name,
      description: description ?? this.description,
      workspaceId: workspaceId ?? this.workspaceId,
      terms: terms ?? this.terms,
      globalOperator: globalOperator ?? this.globalOperator,
      timeRange: timeRange ?? this.timeRange,
      levels: levels ?? this.levels,
      filePattern: filePattern ?? this.filePattern,
      isDefault: isDefault ?? this.isDefault,
      sortOrder: sortOrder ?? this.sortOrder,
      usageCount: usageCount ?? this.usageCount,
      createdAt: createdAt ?? this.createdAt,
      lastUsedAt: lastUsedAt ?? this.lastUsedAt,
    );
  }

  @override
  bool operator ==(Object other) =>
      identical(this, other) ||
      other is SavedFilter &&
          runtimeType == other.runtimeType &&
          id == other.id &&
          workspaceId == other.workspaceId;

  @override
  int get hashCode => id.hashCode ^ workspaceId.hashCode;

  @override
  String toString() =>
      'SavedFilter(id: $id, name: $name, workspaceId: $workspaceId)';
}

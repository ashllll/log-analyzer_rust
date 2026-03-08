import 'dart:convert';

import 'package:uuid/uuid.dart';

import '../models/search.dart';
import '../../core/constants/app_constants.dart';

/// 搜索查询构建器
///
/// 对应 React 版本的 SearchQueryBuilder.ts
/// 提供流畅 API 构建结构化搜索查询
class SearchQueryBuilder {
  final SearchQuery _query;
  final Uuid _uuid = const Uuid();

  SearchQueryBuilder._({
    required String id,
    required List<SearchTerm> terms,
    required QueryOperatorData globalOperator,
    SearchFilters? filters,
    QueryMetadata? metadata,
  }) : _query = SearchQuery(
         id: id,
         terms: terms,
         globalOperator: globalOperator,
         filters: filters,
         metadata:
             metadata ??
             QueryMetadata(
               createdAt: DateTime.now().toIso8601String(),
               updatedAt: null,
               lastUsedAt: null,
               version: null,
             ),
       );

  /// 创建新的查询构建器
  static SearchQueryBuilder create() {
    return SearchQueryBuilder._(
      id: const Uuid().v4(),
      terms: [],
      globalOperator: const QueryOperatorData(value: 'OR'),
      filters: null,
      metadata: null,
    );
  }

  /// 从查询字符串创建构建器
  ///
  /// 解析简单查询字符串（如 "error | timeout"）为结构化查询
  static SearchQueryBuilder fromString(
    String queryString, [
    List<KeywordGroup>? keywordGroups,
  ]) {
    final builder = SearchQueryBuilder.create();

    // 解析查询字符串
    final parts = queryString.split('|');
    for (final part in parts) {
      final term = part.trim();
      if (term.isNotEmpty) {
        builder.addTerm(term, operator_: QueryOperator.or);
      }
    }

    return builder;
  }

  /// 从 JSON 导入查询
  ///
  /// 反序列化 JSON 字符串为 SearchQueryBuilder
  /// 支持由 [export] 方法生成的 JSON 格式
  static SearchQueryBuilder import(String json) {
    try {
      final Map<String, dynamic> decoded = jsonDecode(json);
      final query = SearchQuery.fromJson(decoded);

      return SearchQueryBuilder._(
        id: query.id,
        terms: query.terms,
        globalOperator: query.globalOperator,
        filters: query.filters,
        metadata: query.metadata,
      );
    } catch (e) {
      // 解析失败时返回空构建器
      return SearchQueryBuilder.create();
    }
  }

  /// 添加搜索术语
  SearchQueryBuilder addTerm(
    String value, {
    QueryOperator operator_ = QueryOperator.or,
    bool isRegex = false,
    int priority = 0,
    bool enabled = true,
    bool caseSensitive = false,
    String? presetGroupId,
  }) {
    final term = SearchTerm(
      id: _uuid.v4(),
      value: value,
      operator_: const QueryOperatorData(value: 'OR'),
      source: const TermSourceData(value: 'user'),
      presetGroupId: presetGroupId,
      isRegex: isRegex,
      priority: priority,
      enabled: enabled,
      caseSensitive: caseSensitive,
    );

    return SearchQueryBuilder._(
      id: _query.id,
      terms: [..._query.terms, term],
      globalOperator: _query.globalOperator,
      filters: _query.filters,
      metadata: _query.metadata,
    );
  }

  /// 移除搜索术语
  SearchQueryBuilder removeTerm(String termId) {
    return SearchQueryBuilder._(
      id: _query.id,
      terms: _query.terms.where((t) => t.id != termId).toList(),
      globalOperator: _query.globalOperator,
      filters: _query.filters,
      metadata: _query.metadata,
    );
  }

  /// 切换术语启用状态
  SearchQueryBuilder toggleTerm(String termId) {
    return SearchQueryBuilder._(
      id: _query.id,
      terms: _query.terms
          .map((t) => t.id == termId ? t.copyWith(enabled: !t.enabled) : t)
          .toList(),
      globalOperator: _query.globalOperator,
      filters: _query.filters,
      metadata: _query.metadata,
    );
  }

  /// 清空所有术语
  SearchQueryBuilder clear() {
    return SearchQueryBuilder._(
      id: _query.id,
      terms: [],
      globalOperator: _query.globalOperator,
      filters: _query.filters,
      metadata: _query.metadata,
    );
  }

  /// 设置全局操作符
  SearchQueryBuilder setGlobalOperator(QueryOperator operator_) {
    return SearchQueryBuilder._(
      id: _query.id,
      terms: _query.terms,
      globalOperator: QueryOperatorData(value: operator_.value),
      filters: _query.filters,
      metadata: _query.metadata,
    );
  }

  /// 设置过滤器
  SearchQueryBuilder setFilters(SearchFilters filters) {
    return SearchQueryBuilder._(
      id: _query.id,
      terms: _query.terms,
      globalOperator: _query.globalOperator,
      filters: filters,
      metadata: _query.metadata,
    );
  }

  /// 验证查询
  QueryValidation validate() {
    final errors = <String>[];
    final warnings = <String>[];

    // 检查是否有术语
    if (_query.terms.isEmpty) {
      errors.add('查询不能为空');
    }

    // 检查是否有启用的术语
    final enabledTerms = _query.terms.where((t) => t.enabled).toList();
    if (enabledTerms.isEmpty) {
      warnings.add('没有启用的搜索术语');
    }

    // 检查正则表达式语法
    for (final term in _query.terms) {
      if (term.isRegex && term.enabled) {
        try {
          RegExp(term.value);
        } catch (e) {
          errors.add('无效的正则表达式: ${term.value}');
        }
      }
    }

    return QueryValidation(
      valid: errors.isEmpty,
      errors: errors.isEmpty ? null : errors,
      warnings: warnings.isEmpty ? null : warnings,
    );
  }

  /// 转换为查询字符串
  ///
  /// 生成传统格式的查询字符串（如 "error | timeout | warning"）
  String toQueryString() {
    final enabledTerms = _query.terms.where((t) => t.enabled).toList();
    final parts = enabledTerms.map((t) => t.value);
    return parts.join(_query.globalOperator.value == 'OR' ? ' | ' : ' ');
  }

  /// 转换为优化后的查询
  OptimizedQuery toOptimizedQuery() {
    final enabledTerms = _query.terms.where((t) => t.enabled).toList();

    // 按优先级排序
    final sortedTerms = List<SearchTerm>.from(enabledTerms)
      ..sort((a, b) => b.priority.compareTo(a.priority));

    final prioritizedTerms = sortedTerms.map((t) => t.value).toList();

    // 提取排除术语（NOT 操作符）
    final excludedTerms = sortedTerms
        .where((t) => t.operator_.value == 'NOT')
        .map((t) => t.value)
        .toList();

    return OptimizedQuery(
      queryString: toQueryString(),
      prioritizedTerms: prioritizedTerms,
      excludedTerms: excludedTerms,
      isCaseSensitive: enabledTerms.any((t) => t.caseSensitive) ? true : null,
    );
  }

  /// 导出为 JSON
  ///
  /// 序列化当前查询为 JSON 字符串
  /// 可用于保存查询或通过 [import] 方法恢复
  String export() {
    final query = toQuery();
    const encoder = JsonEncoder.withIndent('  ');
    return encoder.convert(query.toJson());
  }

  /// 获取构建的查询
  SearchQuery toQuery() {
    return _query.copyWith(
      metadata: _query.metadata.copyWith(
        updatedAt: DateTime.now().toIso8601String(),
        lastUsedAt: DateTime.now().toIso8601String(),
      ),
    );
  }

  /// 获取查询 ID
  String get id => _query.id;

  /// 获取术语数量
  int get termCount => _query.terms.length;

  /// 获取启用的术语数量
  int get enabledTermCount => _query.terms.where((t) => t.enabled).length;
}

/// 关键词组模型（用于查询构建）
class KeywordGroup {
  final String id;
  final String name;
  final String color;
  final List<KeywordPattern> patterns;
  final bool enabled;

  const KeywordGroup({
    required this.id,
    required this.name,
    required this.color,
    required this.patterns,
    this.enabled = true,
  });
}

/// 关键词模式模型
class KeywordPattern {
  final String regex;
  final String comment;

  const KeywordPattern({required this.regex, this.comment = ''});
}

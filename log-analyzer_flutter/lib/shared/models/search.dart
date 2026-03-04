import 'package:freezed_annotation/freezed_annotation.dart';

import 'common.dart';

part 'search.freezed.dart';
part 'search.g.dart';

/// 搜索查询模型
///
/// 对应 Rust 后端的 SearchQuery 结构体
/// 对应 React 版本的 src/types/search.ts
@freezed
abstract class SearchQuery with _$SearchQuery {
  const factory SearchQuery({
    required String id,
    required List<SearchTerm> terms,
    @JsonKey(name: 'global_operator') required QueryOperatorData globalOperator,
    SearchFilters? filters,
    required QueryMetadata metadata,
  }) = _SearchQuery;

  factory SearchQuery.fromJson(Map<String, dynamic> json) =>
      _$SearchQueryFromJson(json);
}

/// 搜索术语模型
///
/// 对应 Rust 后端的 SearchTerm 结构体
@freezed
abstract class SearchTerm with _$SearchTerm {
  const factory SearchTerm({
    required String id,
    required String value,
    required QueryOperatorData operator_,
    required TermSourceData source,
    @JsonKey(name: 'preset_group_id') String? presetGroupId,
    @JsonKey(name: 'is_regex') required bool isRegex,
    required int priority,
    required bool enabled,
    @JsonKey(name: 'case_sensitive') required bool caseSensitive,
  }) = _SearchTerm;

  factory SearchTerm.fromJson(Map<String, dynamic> json) =>
      _$SearchTermFromJson(json);
}

/// 查询操作符数据
@freezed
abstract class QueryOperatorData with _$QueryOperatorData {
  const factory QueryOperatorData({
    required String value,
  }) = _QueryOperatorData;

  factory QueryOperatorData.fromJson(Map<String, dynamic> json) =>
      _$QueryOperatorDataFromJson(json);
}

/// 术语来源数据
@freezed
abstract class TermSourceData with _$TermSourceData {
  const factory TermSourceData({
    required String value,
  }) = _TermSourceData;

  factory TermSourceData.fromJson(Map<String, dynamic> json) =>
      _$TermSourceDataFromJson(json);
}

/// 搜索过滤器
///
/// 对应 Rust 后端的 SearchFilters 结构体
@freezed
abstract class SearchFilters with _$SearchFilters {
  const factory SearchFilters({
    List<String>? levels,
    @JsonKey(name: 'time_range') TimeRange? timeRange,
    @JsonKey(name: 'file_pattern') String? filePattern,
  }) = _SearchFilters;

  factory SearchFilters.fromJson(Map<String, dynamic> json) =>
      _$SearchFiltersFromJson(json);
}

/// 查询元数据
@freezed
abstract class QueryMetadata with _$QueryMetadata {
  const factory QueryMetadata({
    @JsonKey(name: 'created_at') required String createdAt,
    @JsonKey(name: 'updated_at') String? updatedAt,
    @JsonKey(name: 'last_used_at') String? lastUsedAt,
    int? version,
  }) = _QueryMetadata;

  factory QueryMetadata.fromJson(Map<String, dynamic> json) =>
      _$QueryMetadataFromJson(json);
}

/// 搜索结果摘要
///
/// 对应 React 版本的 SearchResultSummary
@freezed
abstract class SearchResultSummary with _$SearchResultSummary {
  const factory SearchResultSummary({
    @JsonKey(name: 'total_count') required int totalCount,
    @JsonKey(name: 'match_count') required int matchCount,
    @JsonKey(name: 'duration_ms') required int durationMs,
    @JsonKey(name: 'search_id') required String searchId,
    @JsonKey(name: 'keyword_stats') required List<KeywordStatistic> keywordStats,
  }) = _SearchResultSummary;

  factory SearchResultSummary.fromJson(Map<String, dynamic> json) =>
      _$SearchResultSummaryFromJson(json);
}

/// 关键词统计
///
/// 对应 React 版本的 KeywordStatistics
@freezed
abstract class KeywordStatistic with _$KeywordStatistic {
  const factory KeywordStatistic({
    required String keyword,
    @JsonKey(name: 'match_count') required int matchCount,
    @JsonKey(name: 'match_percentage') required double matchPercentage,
  }) = _KeywordStatistic;

  factory KeywordStatistic.fromJson(Map<String, dynamic> json) =>
      _$KeywordStatisticFromJson(json);
}

/// 搜索参数
///
/// 用于 API 调用的搜索请求参数
@freezed
abstract class SearchParams with _$SearchParams {
  const factory SearchParams({
    required String query,
    @JsonKey(name: 'workspace_id') String? workspaceId,
    @JsonKey(name: 'max_results') int? maxResults,
    SearchFilters? filters,
  }) = _SearchParams;

  factory SearchParams.fromJson(Map<String, dynamic> json) =>
      _$SearchParamsFromJson(json);
}

/// 查询验证结果
///
/// 对应 React 版本的 QueryValidation
@freezed
abstract class QueryValidation with _$QueryValidation {
  const factory QueryValidation({
    required bool valid,
    List<String>? errors,
    List<String>? warnings,
  }) = _QueryValidation;

  factory QueryValidation.fromJson(Map<String, dynamic> json) =>
      _$QueryValidationFromJson(json);
}

/// 优化后的查询
///
/// 对应 React 版本的 OptimizedQuery
@freezed
abstract class OptimizedQuery with _$OptimizedQuery {
  const factory OptimizedQuery({
    required String queryString,
    required List<String> prioritizedTerms,
    required List<String> excludedTerms,
    bool? isCaseSensitive,
  }) = _OptimizedQuery;

  factory OptimizedQuery.fromJson(Map<String, dynamic> json) =>
      _$OptimizedQueryFromJson(json);
}

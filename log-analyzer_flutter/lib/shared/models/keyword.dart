import 'package:freezed_annotation/freezed_annotation.dart';

part 'keyword.freezed.dart';
part 'keyword.g.dart';

/// 关键词模式
///
/// 对应 React 版本的 KeywordPattern
@freezed
abstract class KeywordPattern with _$KeywordPattern {
  const factory KeywordPattern({
    required String regex,
    required String comment,
  }) = _KeywordPattern;

  factory KeywordPattern.fromJson(Map<String, dynamic> json) =>
      _$KeywordPatternFromJson(json);
}

/// 关键词组
///
/// 对应 React 版本的 KeywordGroup
@freezed
abstract class KeywordGroup with _$KeywordGroup {
  const factory KeywordGroup({
    required String id,
    required String name,
    required ColorKeyData color,
    required List<KeywordPattern> patterns,
    required bool enabled,
  }) = _KeywordGroup;

  factory KeywordGroup.fromJson(Map<String, dynamic> json) =>
      _$KeywordGroupFromJson(json);
}

/// 颜色键数据
@freezed
abstract class ColorKeyData with _$ColorKeyData {
  const factory ColorKeyData({
    required String value,
  }) = _ColorKeyData;

  factory ColorKeyData.fromJson(Map<String, dynamic> json) =>
      _$ColorKeyDataFromJson(json);
}

/// 搜索仓库接口
/// 
/// 定义搜索相关的数据操作

import 'package:fpdart/fpdart.dart';
import '../../core/errors/app_error.dart';
import '../entities/log_entry.dart';

/// 搜索仓库接口
abstract class SearchRepository {
  /// 执行搜索
  /// 
  /// 返回搜索 ID，用于后续获取结果
  AppTask<String> search(SearchParams params);

  /// 执行正则搜索
  AppTask<SearchResult> searchRegex(SearchParams params);

  /// 执行组合搜索（多关键词）
  AppTask<SearchResult> searchStructured(SearchParams params);

  /// 获取搜索结果
  /// 
  /// 通过搜索 ID 获取结果
  AppTask<SearchResult> getResults(String searchId);

  /// 取消搜索
  AppTask<void> cancelSearch(String searchId);

  /// 监听搜索进度
  /// 
  /// 返回搜索结果的实时流
  Stream<SearchResult> watchSearchProgress(String searchId);

  /// 导出搜索结果
  /// 
  /// [searchId] 搜索 ID
  /// [format] 导出格式（json/csv）
  /// [outputPath] 输出路径
  AppTask<String> exportResults({
    required String searchId,
    required String format,
    required String outputPath,
  });

  /// 验证正则表达式
  AppTask<bool> validateRegex(String pattern);

  /// 获取搜索历史
  AppTask<List<SearchHistoryItem>> getSearchHistory({
    String? workspaceId,
    int? limit,
  });

  /// 添加搜索历史
  AppTask<void> addSearchHistory({
    required String query,
    required String workspaceId,
    required int resultCount,
  });

  /// 删除搜索历史
  AppTask<void> deleteSearchHistory({
    required String query,
    required String workspaceId,
  });

  /// 清空搜索历史
  AppTask<void> clearSearchHistory({String? workspaceId});
}

/// 搜索历史项
class SearchHistoryItem {
  final String query;
  final String workspaceId;
  final int resultCount;
  final DateTime searchedAt;

  const SearchHistoryItem({
    required this.query,
    required this.workspaceId,
    required this.resultCount,
    required this.searchedAt,
  });
}

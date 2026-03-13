/// 搜索仓库实现
/// 
/// 实现 Domain 层定义的接口

import 'dart:async';

import 'package:fpdart/fpdart.dart';

import '../../core/errors/app_error.dart';
import '../../domain/entities/log_entry.dart';
import '../../domain/repositories/search_repository.dart';
import '../datasources/ffi_datasource.dart';
import '../datasources/event_datasource.dart';

/// 搜索仓库实现
class SearchRepositoryImpl implements SearchRepository {
  final FfiDataSource _ffiDataSource;
  final EventDataSource _eventDataSource;

  SearchRepositoryImpl({
    FfiDataSource? ffiDataSource,
    EventDataSource? eventDataSource,
  })  : _ffiDataSource = ffiDataSource ?? FfiDataSource.instance,
        _eventDataSource = eventDataSource ?? EventDataSource.instance;

  @override
  AppTask<String> search(SearchParams params) {
    return TaskEither(() async {
      // 验证参数
      final validationError = params.validate();
      if (validationError != null) {
        return left(ValidationError(message: validationError));
      }

      try {
        final result = await _ffiDataSource.searchLogs(
          query: params.query,
          workspaceId: params.workspaceId,
          maxResults: params.maxResults,
        ).run();

        return result.fold(
          (error) => left(error),
          (searchId) => right(searchId),
        );
      } catch (e, stack) {
        return left(UnknownError(
          message: '搜索失败',
          technicalDetails: e.toString(),
          cause: e,
        ));
      }
    });
  }

  @override
  AppTask<SearchResult> searchRegex(SearchParams params) {
    return TaskEither(() async {
      try {
        final result = await _ffiDataSource.searchRegex(
          pattern: params.query,
          workspaceId: params.workspaceId,
          maxResults: params.maxResults,
          caseSensitive: params.caseSensitive,
        ).run();

        return result.fold(
          (error) => left(error),
          (entries) => right(SearchResult(
            searchId: DateTime.now().millisecondsSinceEpoch.toString(),
            totalMatches: entries.length,
            entries: entries.map((e) => _mapSearchEntry(e)).toList(),
            isComplete: true,
          )),
        );
      } catch (e, stack) {
        return left(UnknownError(
          message: '正则搜索失败',
          technicalDetails: e.toString(),
          cause: e,
        ));
      }
    });
  }

  @override
  AppTask<SearchResult> searchStructured(SearchParams params) {
    // TODO: 实现结构化搜索
    return TaskEither(() async {
      return right(SearchResult.empty);
    });
  }

  @override
  AppTask<SearchResult> getResults(String searchId) {
    // 搜索结果通过事件流实时接收
    return TaskEither(() async {
      return right(SearchResult(searchId: searchId));
    });
  }

  @override
  AppTask<void> cancelSearch(String searchId) {
    return TaskEither(() async {
      final result = await _ffiDataSource.cancelSearch(searchId).run();
      return result.map((_) => null);
    });
  }

  @override
  Stream<SearchResult> watchSearchProgress(String searchId) {
    return _eventDataSource.searchResults
        .where((result) => result.searchId == searchId);
  }

  @override
  AppTask<String> exportResults({
    required String searchId,
    required String format,
    required String outputPath,
  }) {
    // TODO: 实现导出
    return TaskEither(() async {
      return right('');
    });
  }

  @override
  AppTask<bool> validateRegex(String pattern) {
    return TaskEither(() async {
      final result = await _ffiDataSource.validateRegex(pattern).run();
      return result.map((r) => r.valid);
    });
  }

  @override
  AppTask<List<SearchHistoryItem>> getSearchHistory({
    String? workspaceId,
    int? limit,
  }) {
    // TODO: 实现搜索历史
    return TaskEither(() async {
      return right([]);
    });
  }

  @override
  AppTask<void> addSearchHistory({
    required String query,
    required String workspaceId,
    required int resultCount,
  }) {
    // TODO: 实现添加搜索历史
    return TaskEither(() async {
      return right(null);
    });
  }

  @override
  AppTask<void> deleteSearchHistory({
    required String query,
    required String workspaceId,
  }) {
    // TODO: 实现删除搜索历史
    return TaskEither(() async {
      return right(null);
    });
  }

  @override
  AppTask<void> clearSearchHistory({String? workspaceId}) {
    // TODO: 实现清空搜索历史
    return TaskEither(() async {
      return right(null);
    });
  }

  LogEntry _mapSearchEntry(dynamic entry) {
    // TODO: 实现完整的映射逻辑
    return LogEntry.empty;
  }
}

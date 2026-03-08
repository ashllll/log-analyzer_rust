import 'package:flutter/material.dart';

/// 应用本地化类
///
/// 对应 React 版本的 i18next 配置
/// 使用 ARB 文件格式
class AppLocalizations {
  const AppLocalizations(this.locale);

  final Locale locale;

  static AppLocalizations of(BuildContext context) {
    return Localizations.of<AppLocalizations>(context, AppLocalizations)!;
  }

  static const LocalizationsDelegate<AppLocalizations> delegate =
      _AppLocalizationsDelegate();

  static const List<Locale> supportedLocales = [Locale('zh'), Locale('en')];

  // ==================== 通用 ====================

  String get appTitle => locale.languageCode == 'zh' ? '日志分析器' : 'Log Analyzer';

  // ==================== 搜索 ====================

  String get search_title => locale.languageCode == 'zh' ? '搜索' : 'Search';

  String get search_statistics_title =>
      locale.languageCode == 'zh' ? '搜索统计' : 'Search Statistics';

  String search_total_matches(int count, int duration) =>
      locale.languageCode == 'zh'
      ? '总计: $count 条匹配, 耗时 ${duration}ms'
      : 'Total: $count matches, ${duration}ms';

  String get search_placeholder =>
      locale.languageCode == 'zh' ? '搜索日志...' : 'Search logs...';

  String get search_no_results =>
      locale.languageCode == 'zh' ? '未找到匹配结果' : 'No results found';

  String get search_loading =>
      locale.languageCode == 'zh' ? '搜索中...' : 'Searching...';

  // ==================== 关键词 ====================

  String get keywords_title => locale.languageCode == 'zh' ? '关键词' : 'Keywords';

  String get keywords_add_group =>
      locale.languageCode == 'zh' ? '添加关键词组' : 'Add Keyword Group';

  String get keywords_edit_group =>
      locale.languageCode == 'zh' ? '编辑关键词组' : 'Edit Keyword Group';

  String get keywords_delete_confirm => locale.languageCode == 'zh'
      ? '确定要删除此关键词组吗？'
      : 'Delete this keyword group?';

  String get keywords_name =>
      locale.languageCode == 'zh' ? '组名称' : 'Group Name';

  String get keywords_color => locale.languageCode == 'zh' ? '颜色' : 'Color';

  String get keywords_enabled => locale.languageCode == 'zh' ? '启用' : 'Enabled';

  String get keywords_patterns =>
      locale.languageCode == 'zh' ? '模式' : 'Patterns';

  String get keywords_import => locale.languageCode == 'zh' ? '导入' : 'Import';

  String get keywords_export => locale.languageCode == 'zh' ? '导出' : 'Export';

  // ==================== 工作区 ====================

  String get workspaces_title =>
      locale.languageCode == 'zh' ? '工作区' : 'Workspaces';

  String get workspaces_add =>
      locale.languageCode == 'zh' ? '添加工作区' : 'Add Workspace';

  String get workspaces_delete_confirm =>
      locale.languageCode == 'zh' ? '确定要删除此工作区吗？' : 'Delete this workspace?';

  String get workspaces_import =>
      locale.languageCode == 'zh' ? '导入文件夹' : 'Import Folder';

  String get workspaces_refresh =>
      locale.languageCode == 'zh' ? '刷新' : 'Refresh';

  String get workspaces_watch =>
      locale.languageCode == 'zh' ? '监听文件变化' : 'Watch File Changes';

  String get workspaces_status_ready =>
      locale.languageCode == 'zh' ? '就绪' : 'Ready';

  String get workspaces_status_scanning =>
      locale.languageCode == 'zh' ? '扫描中' : 'Scanning';

  String get workspaces_status_offline =>
      locale.languageCode == 'zh' ? '离线' : 'Offline';

  String get workspaces_status_processing =>
      locale.languageCode == 'zh' ? '处理中' : 'Processing';

  // ==================== 任务 ====================

  String get tasks_title => locale.languageCode == 'zh' ? '任务' : 'Tasks';

  String get tasks_no_tasks =>
      locale.languageCode == 'zh' ? '没有运行中的任务' : 'No running tasks';

  String get tasks_cancel =>
      locale.languageCode == 'zh' ? '取消任务' : 'Cancel Task';

  String get tasks_cancel_confirm =>
      locale.languageCode == 'zh' ? '确定要取消此任务吗？' : 'Cancel this task?';

  // ==================== 设置 ====================

  String get settings_title => locale.languageCode == 'zh' ? '设置' : 'Settings';

  String get settings_save => locale.languageCode == 'zh' ? '保存' : 'Save';

  String get settings_saved =>
      locale.languageCode == 'zh' ? '设置保存成功' : 'Settings saved successfully';

  String get settings_save_failed =>
      locale.languageCode == 'zh' ? '设置保存失败' : 'Failed to save settings';

  // ==================== 性能 ====================

  String get performance_title =>
      locale.languageCode == 'zh' ? '性能' : 'Performance';

  String get performance_search_latency =>
      locale.languageCode == 'zh' ? '搜索延迟' : 'Search Latency';

  String get performance_search_throughput =>
      locale.languageCode == 'zh' ? '搜索吞吐量' : 'Search Throughput';

  String get performance_cache_metrics =>
      locale.languageCode == 'zh' ? '缓存指标' : 'Cache Metrics';

  String get performance_memory_metrics =>
      locale.languageCode == 'zh' ? '内存指标' : 'Memory Metrics';

  String get performance_task_metrics =>
      locale.languageCode == 'zh' ? '任务指标' : 'Task Metrics';

  String get performance_index_metrics =>
      locale.languageCode == 'zh' ? '索引指标' : 'Index Metrics';

  // ==================== Toast ====================

  String get toast_success => locale.languageCode == 'zh' ? '成功' : 'Success';

  String get toast_error => locale.languageCode == 'zh' ? '错误' : 'Error';

  String get toast_info => locale.languageCode == 'zh' ? '信息' : 'Info';

  String get toast_warning => locale.languageCode == 'zh' ? '警告' : 'Warning';

  // ==================== 通用 ====================

  String get common_confirm => locale.languageCode == 'zh' ? '确认' : 'Confirm';

  String get common_cancel => locale.languageCode == 'zh' ? '取消' : 'Cancel';

  String get common_delete => locale.languageCode == 'zh' ? '删除' : 'Delete';

  String get common_edit => locale.languageCode == 'zh' ? '编辑' : 'Edit';

  String get common_save => locale.languageCode == 'zh' ? '保存' : 'Save';

  String get common_close => locale.languageCode == 'zh' ? '关闭' : 'Close';

  String get common_loading =>
      locale.languageCode == 'zh' ? '加载中...' : 'Loading...';

  String get common_no_data => locale.languageCode == 'zh' ? '暂无数据' : 'No Data';
}

/// 本地化委托类
class _AppLocalizationsDelegate
    extends LocalizationsDelegate<AppLocalizations> {
  const _AppLocalizationsDelegate();

  @override
  Future<AppLocalizations> load(Locale locale) async {
    return AppLocalizations(locale);
  }

  @override
  bool isSupported(Locale locale) {
    return AppLocalizations.supportedLocales.any(
      (supported) => supported.languageCode == locale.languageCode,
    );
  }

  @override
  bool shouldReload(covariant _AppLocalizationsDelegate old) {
    // Delegate 是无状态的，不需要重新加载
    return false;
  }
}

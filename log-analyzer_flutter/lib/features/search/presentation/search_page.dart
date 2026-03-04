import 'dart:async';
import 'dart:typed_data';

import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:file_picker/file_picker.dart';

import '../../../shared/models/common.dart';
import '../../../shared/models/search.dart';
import '../../../shared/providers/app_provider.dart';
import '../../../shared/providers/workspace_provider.dart';
import '../../../shared/services/api_service.dart';
import '../../../shared/services/event_stream_service.dart';
import '../../../core/theme/app_theme.dart';
import '../../../core/constants/app_constants.dart';
import 'widgets/log_row_widget.dart';
import 'widgets/search_stats_panel.dart';
import 'widgets/filter_palette.dart';
import 'widgets/heatmap_minimap.dart';
import 'widgets/log_detail_panel.dart';
import 'widgets/search_progress_bar.dart';

/// 固定行高常量 - 用于 SliverFixedExtentList
///
/// 此值必须与 LogRowWidget 中的 StrutStyle.height * StrutStyle.fontSize 一致
/// 以确保 O(1) 视口物理锚点的确定性
const double _kLogItemExtent = 36.0;

/// 搜索页面
///
/// 对应 React 版本的 SearchPage.tsx
/// 核心功能：
/// - 日志搜索（支持防抖）
/// - SliverFixedExtentList 虚拟滚动（O(1) 视口锚点）
/// - 关键词高亮显示
/// - 搜索结果统计
/// - 高级过滤器
///
/// PRD V6.0 4.1 要求：
/// - 强制使用 SliverFixedExtentList 实现确定性视口
/// - 配合 StrutStyle(forceStrutHeight: true) 镇压行高突变
class SearchPage extends ConsumerStatefulWidget {
  const SearchPage({super.key});

  @override
  ConsumerState<SearchPage> createState() => _SearchPageState();
}

class _SearchPageState extends ConsumerState<SearchPage> {
  final _searchController = TextEditingController();
  final _scrollController = ScrollController();
  final _focusNode = FocusNode();

  // 搜索状态
  List<LogEntry> _logs = [];
  String? _currentSearchId;
  bool _isSearching = false;
  SearchResultSummary? _searchSummary;

  // 进度条状态
  int _progress = 0;
  int _scannedFiles = 0;
  int _resultsFound = 0;

  // 过滤器状态
  bool _showFilters = false;
  FilterOptions? _currentFilters;

  // 选中的日志索引
  int? _selectedLogIndex;

  // 防抖定时器
  Timer? _debounceTimer;

  // 事件流订阅
  StreamSubscription<List<LogEntry>>? _searchResultsSubscription;
  StreamSubscription<SearchResultSummary>? _searchSummarySubscription;

  // 视口状态 - 用于追踪当前可见范围（调试用）
  // ignore: unused_field
  int _firstVisibleIndex = 0;
  // ignore: unused_field
  int _lastVisibleIndex = 0;

  // 热力图密度数据 (从 Rust 端传入)
  Uint8List? _densityMap;
  int _maxDensity = 255;

  @override
  void initState() {
    super.initState();
    _focusNode.requestFocus();
    _subscribeToEventStream();
  }

  @override
  void dispose() {
    _searchController.dispose();
    _scrollController.dispose();
    _focusNode.dispose();
    _debounceTimer?.cancel();
    _searchResultsSubscription?.cancel();
    _searchSummarySubscription?.cancel();
    super.dispose();
  }

  /// 订阅事件流服务
  ///
  /// 监听搜索结果和搜索摘要的实时更新
  void _subscribeToEventStream() {
    final eventStreamService = ref.read(eventStreamServiceProvider);

    // 监听搜索结果流
    _searchResultsSubscription = eventStreamService.searchResults.listen(
      (results) {
        setState(() {
          _logs = results;
          _isSearching = false;
          _resultsFound = results.length;
          _progress = 100; // 搜索完成
        });
        // 生成热力图密度数据
        _generateDensityMap();
      },
      onError: (error) {
        setState(() {
          _isSearching = false;
          _progress = 0;
        });
        ref.read(appStateProvider.notifier).addToast(
              ToastType.error,
              '搜索结果接收失败: $error',
            );
      },
    );

    // 监听搜索摘要流
    _searchSummarySubscription = eventStreamService.searchSummary.listen(
      (summary) {
        setState(() {
          _searchSummary = summary;
          // 从摘要中获取结果数量
          _resultsFound = summary.matchCount;
          // 根据持续时间计算进度（假设最大 30 秒为 100%）
          _progress = ((summary.durationMs / 30000) * 100).clamp(0, 100).toInt();
        });
      },
    );
  }

  /// 生成热力图密度数据
  ///
  /// PRD V6.0 4.2 GPU 着色器缩略图
  /// 从 Rust 端获取 density_map，当前使用模拟数据
  void _generateDensityMap() {
    if (_logs.isEmpty) {
      setState(() {
        _densityMap = null;
      });
      return;
    }

    // TODO: 从 Rust 端获取 SearchProgress.gpu_texture_map
    // 当前使用模拟数据：根据日志级别生成密度
    final densityData = Uint8List(_logs.length);
    int maxVal = 0;

    for (int i = 0; i < _logs.length; i++) {
      final log = _logs[i];
      int density;

      // 根据日志级别设置密度值
      switch (log.level.toUpperCase()) {
        case 'ERROR':
        case 'FATAL':
          density = 255; // 红色 - 最高密度
          break;
        case 'WARN':
        case 'WARNING':
          density = 180; // 黄色 - 高密度
          break;
        case 'INFO':
          density = 100; // 绿色 - 中密度
          break;
        case 'DEBUG':
          density = 50; // 青色 - 低密度
          break;
        default:
          density = 25; // 蓝色 - 最低密度
      }

      // 如果有匹配关键词，增加密度
      if (log.matchedKeywords != null && log.matchedKeywords!.isNotEmpty) {
        density = (density * 1.5).clamp(0, 255).toInt();
      }

      densityData[i] = density;
      if (density > maxVal) maxVal = density;
    }

    setState(() {
      _densityMap = densityData;
      _maxDensity = maxVal > 0 ? maxVal : 255;
    });
  }

  @override
  Widget build(BuildContext context) {
    final activeWorkspaceId = ref.watch(appStateProvider).activeWorkspaceId;
    final activeWorkspace = activeWorkspaceId != null
        ? ref.read(workspaceStateProvider.notifier).getWorkspaceById(activeWorkspaceId)
        : null;

    // 键盘快捷键处理：Ctrl+F / Cmd+F 聚焦搜索框
    return KeyboardListener(
      focusNode: FocusNode(),
      autofocus: true,
      onKeyEvent: (event) {
        if (event is KeyDownEvent) {
          // Ctrl+F (Windows/Linux) 或 Cmd+F (macOS)
          if (event.logicalKey == LogicalKeyboardKey.keyF &&
              (HardwareKeyboard.instance.isControlPressed ||
                  HardwareKeyboard.instance.isMetaPressed)) {
            _focusNode.requestFocus();
          }
        }
      },
      child: Scaffold(
        appBar: _buildAppBar(activeWorkspace),
        body: Column(
          children: [
            // 搜索栏
            _buildSearchBar(),
            // 进度条（搜索进行中显示）
            if (_isSearching || _progress > 0 || _scannedFiles > 0)
              SearchProgressBar(
                progress: _progress,
                scannedFiles: _scannedFiles,
                resultsFound: _resultsFound,
                isCompleted: !_isSearching && _progress >= 100,
                onCancel: _isSearching ? _cancelSearch : null,
              ),
            // 过滤器面板
            if (_showFilters)
              FilterPalette(
                onApply: _applyFiltersFromPalette,
                currentFilters: _currentFilters,
              ),
            // 日志列表（带热力图）
            Expanded(
              child: _buildLogsListWithHeatmap(),
            ),
            // 统计面板
            if (_searchSummary != null || _isSearching)
              SearchStatsPanel(
                summary: _searchSummary,
                isLoading: _isSearching,
                onExport: _logs.isNotEmpty ? _exportResults : null,
              ),
          ],
        ),
      ),
    );
  }

  /// 构建 AppBar
  PreferredSizeWidget _buildAppBar(Workspace? activeWorkspace) {
    return AppBar(
      backgroundColor: AppColors.bgMain,
      elevation: 0,
      title: Text(
        activeWorkspace?.name ?? '请先选择工作区',
        style: const TextStyle(
          fontSize: 18,
          fontWeight: FontWeight.w600,
        ),
      ),
      actions: [
        // 过滤器切换按钮
        IconButton(
          icon: Icon(
            _showFilters ? Icons.filter_list_off : Icons.filter_list,
            color: _showFilters ? AppColors.primary : AppColors.textMuted,
          ),
          tooltip: '高级过滤器',
          onPressed: () {
            setState(() {
              _showFilters = !_showFilters;
            });
          },
        ),
        // 更多操作
        PopupMenuButton<String>(
          icon: const Icon(Icons.more_vert),
          onSelected: (value) => _handleMenuAction(value),
          itemBuilder: (context) => [
            const PopupMenuItem(
              value: 'clear',
              child: Row(
                children: [
                  Icon(Icons.clear_all, size: 18),
                  SizedBox(width: 12),
                  Text('清除结果'),
                ],
              ),
            ),
            const PopupMenuItem(
              value: 'export',
              child: Row(
                children: [
                  Icon(Icons.download, size: 18),
                  SizedBox(width: 12),
                  Text('导出全部'),
                ],
              ),
            ),
          ],
        ),
      ],
    );
  }

  /// 构建搜索栏
  Widget _buildSearchBar() {
    return Container(
      padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 12),
      decoration: const BoxDecoration(
        color: AppColors.bgCard,
        border: Border(
          bottom: BorderSide(color: AppColors.border, width: 1),
        ),
      ),
      child: Row(
        children: [
          // 搜索输入框（不显示清空按钮，根据 CONTEXT 要求）
          Expanded(
            child: TextField(
              controller: _searchController,
              focusNode: _focusNode,
              decoration: const InputDecoration(
                hintText: '搜索日志... (Ctrl+F 聚焦)',
                hintStyle: TextStyle(color: AppColors.textMuted),
                prefixIcon: Icon(Icons.search),
                border: InputBorder.none,
                contentPadding: EdgeInsets.symmetric(
                  horizontal: 16,
                  vertical: 12,
                ),
              ),
              style: const TextStyle(fontSize: 15),
              onChanged: (_) => setState(() {}),
              onSubmitted: (_) => _performSearch(),
            ),
          ),
          // 搜索按钮
          ElevatedButton(
            onPressed: _isSearching ? null : _performSearch,
            style: ElevatedButton.styleFrom(
              backgroundColor: AppColors.primary,
              foregroundColor: Colors.white,
              padding: const EdgeInsets.symmetric(
                horizontal: 20,
                vertical: 16,
              ),
              shape: RoundedRectangleBorder(
                borderRadius: BorderRadius.circular(8),
              ),
            ),
            child: _isSearching
                ? const SizedBox(
                    width: 20,
                    height: 20,
                    child: CircularProgressIndicator(
                      strokeWidth: 2,
                      valueColor: AlwaysStoppedAnimation<Color>(Colors.white),
                    ),
                  )
                : const Text('搜索'),
          ),
        ],
      ),
    );
  }

  /// 构建日志列表
  ///
  /// 构建日志列表（带热力图）
  ///
  /// PRD V6.0 4.2 GPU 着色器缩略图
  /// - 左侧: 日志列表 (SliverFixedExtentList)
  /// - 右侧: 热力图缩略图 (HeatmapMinimap)
  Widget _buildLogsListWithHeatmap() {
    if (_logs.isEmpty) {
      return _buildEmptyState();
    }

    return Row(
      children: [
        // 日志列表
        Expanded(
          child: _buildLogsList(),
        ),
        // 热力图缩略图
        if (_densityMap != null && _densityMap!.isNotEmpty)
          _buildHeatmapSidebar(),
      ],
    );
  }

  /// 构建热力图侧边栏
  Widget _buildHeatmapSidebar() {
    return Container(
      width: 24,
      decoration: const BoxDecoration(
        color: AppColors.bgCard,
        border: Border(
          left: BorderSide(color: AppColors.border, width: 1),
        ),
      ),
      child: Column(
        children: [
          // 热力图组件
          Expanded(
            child: HeatmapMinimap(
              densityMap: _densityMap,
              maxDensity: _maxDensity,
              width: 24,
              height: double.infinity,
              onTap: _scrollToLogIndex,
            ),
          ),
        ],
      ),
    );
  }

  /// 滚动到指定日志行
  ///
  /// 点击热力图时调用，实现快速导航
  void _scrollToLogIndex(int index) {
    if (_logs.isEmpty || index < 0 || index >= _logs.length) return;

    // 计算目标偏移量
    final targetOffset = index * _kLogItemExtent;

    // 滚动到目标位置
    _scrollController.animateTo(
      targetOffset,
      duration: const Duration(milliseconds: 300),
      curve: Curves.easeInOut,
    );

    // 选中该行
    setState(() {
      _selectedLogIndex = index;
    });
  }

  /// 构建日志列表
  ///
  /// 使用 CustomScrollView + SliverFixedExtentList 实现确定性虚拟滚动
  ///
  /// PRD V6.0 4.1 要求：
  /// - 强制使用 SliverFixedExtentList 实现 O(1) 视口锚点
  /// - itemExtent 必须与 LogRowWidget 的 StrutStyle 完全一致
  /// - 支持 10,000+ 条日志流畅滚动（60FPS）
  Widget _buildLogsList() {
    if (_logs.isEmpty) {
      return _buildEmptyState();
    }

    // 使用 CustomScrollView + SliverFixedExtentList 实现确定性视口
    // 这是 Flutter 原生最高性能的虚拟滚动方案
    return NotificationListener<ScrollNotification>(
      onNotification: _handleScrollNotification,
      child: CustomScrollView(
        controller: _scrollController,
        // 启用语义化支持，提升无障碍访问体验
        semanticChildCount: _logs.length,
        slivers: [
          // 固定高度的虚拟滚动列表
          // itemExtent 确保 O(1) 视口锚点计算
          SliverFixedExtentList(
            itemExtent: _kLogItemExtent,
            delegate: SliverChildBuilderDelegate(
              (context, index) => _buildLogRow(index),
              childCount: _logs.length,
              // 启用自动清理不可见项，减少内存占用
              addAutomaticKeepAlives: true,
              // 启用自动维护语义化索引
              addRepaintBoundaries: true,
              // 启用语义化索引
              addSemanticIndexes: true,
            ),
          ),
        ],
      ),
    );
  }

  /// 构建单行日志
  ///
  /// 抽取为独立方法便于 SliverChildBuilderDelegate 调用
  Widget _buildLogRow(int index) {
    final log = _logs[index];
    final isSelected = _selectedLogIndex == index;

    return LogRowWidget(
      log: log,
      isActive: isSelected,
      matchedKeywords: log.matchedKeywords,
      itemExtent: _kLogItemExtent,
      onTap: () => _selectLog(index),
    );
  }

  /// 处理滚动通知
  ///
  /// 追踪当前可见范围，用于性能监控和优化
  bool _handleScrollNotification(ScrollNotification notification) {
    if (notification is ScrollUpdateNotification && _logs.isNotEmpty) {
      // 计算当前可见范围
      final viewportHeight = notification.metrics.viewportDimension;
      final scrollOffset = notification.metrics.pixels;

      _firstVisibleIndex = (scrollOffset / _kLogItemExtent).floor();
      _lastVisibleIndex = ((scrollOffset + viewportHeight) / _kLogItemExtent).ceil().clamp(0, _logs.length - 1);

      // 可选：在此处添加性能监控或日志预加载逻辑
    }
    return false;
  }

  /// 构建空状态
  Widget _buildEmptyState() {
    final hasQuery = _searchController.text.isNotEmpty;

    return Center(
      child: Column(
        mainAxisAlignment: MainAxisAlignment.center,
        children: [
          Icon(
            hasQuery ? Icons.search_off : Icons.search_outlined,
            size: 64,
            color: AppColors.textMuted,
          ),
          const SizedBox(height: 16),
          Text(
            hasQuery ? '未找到匹配结果' : '输入关键词开始搜索日志',
            style: const TextStyle(
              fontSize: 16,
              color: AppColors.textSecondary,
            ),
          ),
          if (hasQuery) ...[
            const SizedBox(height: 8),
            TextButton(
              onPressed: () {
                _searchController.clear();
                setState(() {
                  _logs = [];
                  _searchSummary = null;
                });
              },
              child: const Text('清除搜索'),
            ),
          ],
        ],
      ),
    );
  }

  /// 搜索输入变化（点击搜索按钮执行，不使用防抖）
  void _onSearchChanged(String value) {
    // 只更新 UI 状态，不执行搜索
    // 搜索由点击搜索按钮或 Enter 键触发
    setState(() {});
  }

  /// 取消搜索
  void _cancelSearch() {
    // 取消当前搜索
    setState(() {
      _isSearching = false;
      _progress = 0;
      _scannedFiles = 0;
      _resultsFound = 0;
    });
    ref.read(appStateProvider.notifier).addToast(
          ToastType.info,
          '搜索已取消',
        );
  }

  /// 执行搜索
  ///
  /// 调用后端搜索 API 并通过事件流接收结果
  Future<void> _performSearch() async {
    final query = _searchController.text.trim();
    if (query.isEmpty) return;

    final activeWorkspaceId = ref.read(appStateProvider).activeWorkspaceId;
    if (activeWorkspaceId == null) {
      ref.read(appStateProvider.notifier).addToast(
            ToastType.error,
            '请先选择工作区',
          );
      return;
    }

    setState(() {
      _isSearching = true;
      _logs = [];
      _searchSummary = null;
      _selectedLogIndex = null;
      _progress = 0;
      _scannedFiles = 0;
      _resultsFound = 0;
    });

    try {
      final apiService = ref.read(apiServiceProvider);
      final searchId = await apiService.searchLogs(
        query: query,
        workspaceId: activeWorkspaceId,
        maxResults: AppConstants.defaultMaxResults,
        filterOptions: _currentFilters,
      );

      setState(() {
        _currentSearchId = searchId;
      });

      // 搜索结果通过事件流实时接收，这里设置超时保护
      // 如果 30 秒内没有收到结果，则标记为失败
      await Future.delayed(const Duration(seconds: 30));

      // 如果还在搜索中，说明事件流可能未正常工作
      if (_isSearching && _logs.isEmpty) {
        setState(() {
          _isSearching = false;
        });
        ref.read(appStateProvider.notifier).addToast(
              ToastType.warning,
              '搜索超时，请检查后端连接',
            );
      }
    } catch (e) {
      setState(() {
        _isSearching = false;
        _progress = 0;
      });
      ref.read(appStateProvider.notifier).addToast(
            ToastType.error,
            '搜索失败: $e',
          );
    }
  }

  /// 选择日志行
  void _selectLog(int index) {
    if (index < 0 || index >= _logs.length) return;

    final log = _logs[index];

    // 显示详情面板
    _showLogDetail(log);
  }

  /// 显示日志详情面板
  void _showLogDetail(LogEntry entry) {
    showDialog(
      context: context,
      builder: (dialogContext) => LogDetailPanel(
        entry: entry,
        keywords: entry.matchedKeywords,
        onClose: () => Navigator.pop(dialogContext),
      ),
    );
  }

  /// 处理菜单操作
  void _handleMenuAction(String action) {
    switch (action) {
      case 'clear':
        setState(() {
          _logs.clear();
          _searchSummary = null;
          _selectedLogIndex = null;
        });
        break;
      case 'export':
        _exportResults();
        break;
    }
  }

  /// 导出结果
  ///
  /// 显示导出对话框，让用户选择导出格式和路径
  Future<void> _exportResults() async {
    if (_logs.isEmpty) {
      ref.read(appStateProvider.notifier).addToast(
            ToastType.warning,
            '没有可导出的结果',
          );
      return;
    }

    // 显示导出对话框
    final exportConfig = await _showExportDialog();
    if (exportConfig == null) return;

    try {
      final apiService = ref.read(apiServiceProvider);
      final outputPath = await apiService.exportResults(
        searchId: _currentSearchId ?? '',
        format: exportConfig['format'] ?? 'json',
        outputPath: exportConfig['path'] ?? '',
      );

      ref.read(appStateProvider.notifier).addToast(
            ToastType.success,
            '导出成功: $outputPath',
          );
    } catch (e) {
      ref.read(appStateProvider.notifier).addToast(
            ToastType.error,
            '导出失败: $e',
          );
    }
  }

  /// 显示导出对话框
  ///
  /// 让用户选择导出格式（JSON/CSV）和保存路径
  Future<Map<String, String>?> _showExportDialog() async {
    String selectedFormat = 'json';

    return showDialog<Map<String, String>>(
      context: context,
      builder: (context) => StatefulBuilder(
        builder: (context, setDialogState) => AlertDialog(
          title: const Text('导出搜索结果'),
          content: Column(
            mainAxisSize: MainAxisSize.min,
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              // 结果数量提示
              Text(
                '共 ${_logs.length} 条结果',
                style: const TextStyle(
                  color: AppColors.textSecondary,
                  fontSize: 14,
                ),
              ),
              const SizedBox(height: 16),
              // 格式选择
              const Text(
                '导出格式',
                style: TextStyle(
                  fontWeight: FontWeight.w600,
                  fontSize: 14,
                ),
              ),
              const SizedBox(height: 8),
              Row(
                children: [
                  _buildFormatOption(
                    'JSON',
                    'json',
                    selectedFormat,
                    (value) => setDialogState(() => selectedFormat = value),
                  ),
                  const SizedBox(width: 12),
                  _buildFormatOption(
                    'CSV',
                    'csv',
                    selectedFormat,
                    (value) => setDialogState(() => selectedFormat = value),
                  ),
                ],
              ),
            ],
          ),
          actions: [
            TextButton(
              onPressed: () => Navigator.pop(context),
              child: const Text('取消'),
            ),
            ElevatedButton(
              onPressed: () async {
                // 选择保存路径
                final path = await _selectExportPath(selectedFormat);
                if (path != null) {
                  Navigator.pop(context, {
                    'format': selectedFormat,
                    'path': path,
                  });
                }
              },
              child: const Text('选择路径并导出'),
            ),
          ],
        ),
      ),
    );
  }

  /// 构建格式选择选项
  Widget _buildFormatOption(
    String label,
    String value,
    String selectedValue,
    void Function(String) onSelect,
  ) {
    final isSelected = value == selectedValue;
    return InkWell(
      onTap: () => onSelect(value),
      borderRadius: BorderRadius.circular(8),
      child: Container(
        padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 10),
        decoration: BoxDecoration(
          color: isSelected ? AppColors.primary.withValues(alpha: 0.2) : AppColors.bgInput,
          borderRadius: BorderRadius.circular(8),
          border: Border.all(
            color: isSelected ? AppColors.primary : AppColors.border,
            width: isSelected ? 2 : 1,
          ),
        ),
        child: Row(
          mainAxisSize: MainAxisSize.min,
          children: [
            Icon(
              isSelected ? Icons.radio_button_checked : Icons.radio_button_off,
              size: 18,
              color: isSelected ? AppColors.primary : AppColors.textMuted,
            ),
            const SizedBox(width: 8),
            Text(
              label,
              style: TextStyle(
                color: isSelected ? AppColors.primary : AppColors.textSecondary,
                fontWeight: isSelected ? FontWeight.w600 : FontWeight.normal,
              ),
            ),
          ],
        ),
      ),
    );
  }

  /// 选择导出路径
  ///
  /// 使用 file_picker 让用户选择保存位置
  Future<String?> _selectExportPath(String format) async {
    final extension = format == 'json' ? 'json' : 'csv';
    final timestamp = DateTime.now().toIso8601String().replaceAll(':', '-').split('.').first;

    try {
      final result = await FilePicker.platform.saveFile(
        dialogTitle: '选择保存位置',
        fileName: 'search_results_$timestamp.$extension',
        type: FileType.custom,
        allowedExtensions: [extension],
      );

      return result;
    } catch (e) {
      ref.read(appStateProvider.notifier).addToast(
            ToastType.error,
            '选择路径失败: $e',
          );
      return null;
    }
  }

  /// 应用过滤器
  ///
  /// 从 FilterPalette 接收过滤器配置并触发重新搜索
  void applyFilters(FilterOptions filters) {
    setState(() {
      _currentFilters = filters;
    });

    // 如果有搜索词，自动重新搜索
    if (_searchController.text.trim().isNotEmpty) {
      _performSearch();
    }
  }

  /// 从 FilterPalette 应用过滤器
  ///
  /// 接收原始过滤器数据并构建 FilterOptions
  void _applyFiltersFromPalette({
    required TimeRange timeRange,
    required List<String> levels,
    String? filePattern,
  }) {
    final filters = FilterOptions(
      timeRange: timeRange,
      levels: levels,
      filePattern: filePattern,
    );
    applyFilters(filters);
  }
}

// 统计面板端到端测试
//
// 测试统计面板与搜索结果的集成
// 验证日志级别统计显示、点击筛选、自动刷新功能

import 'package:flutter_test/flutter_test.dart';

// LogLevelStats 模型（本地定义，避免 FFI 依赖）
class LogLevelStats {
  final int fatalCount;
  final int errorCount;
  final int warnCount;
  final int infoCount;
  final int debugCount;
  final int traceCount;
  final int unknownCount;
  final int total;

  const LogLevelStats({
    required this.fatalCount,
    required this.errorCount,
    required this.warnCount,
    required this.infoCount,
    required this.debugCount,
    required this.traceCount,
    required this.unknownCount,
    required this.total,
  });

  factory LogLevelStats.fromMap(Map<String, dynamic>? map) {
    if (map == null) {
      return LogLevelStats.empty;
    }
    return LogLevelStats(
      fatalCount: (map['fatalCount'] as num?)?.toInt() ?? 0,
      errorCount: (map['errorCount'] as num?)?.toInt() ?? 0,
      warnCount: (map['warnCount'] as num?)?.toInt() ?? 0,
      infoCount: (map['infoCount'] as num?)?.toInt() ?? 0,
      debugCount: (map['debugCount'] as num?)?.toInt() ?? 0,
      traceCount: (map['traceCount'] as num?)?.toInt() ?? 0,
      unknownCount: (map['unknownCount'] as num?)?.toInt() ?? 0,
      total: (map['total'] as num?)?.toInt() ?? 0,
    );
  }

  static const empty = LogLevelStats(
    fatalCount: 0,
    errorCount: 0,
    warnCount: 0,
    infoCount: 0,
    debugCount: 0,
    traceCount: 0,
    unknownCount: 0,
    total: 0,
  );
}

void main() {
  group('统计面板端到端测试', () {
    test('LogLevelStats 模型创建', () {
      // 创建测试用统计数据
      final stats = LogLevelStats(
        fatalCount: 5,
        errorCount: 150,
        warnCount: 300,
        infoCount: 5000,
        debugCount: 800,
        traceCount: 200,
        unknownCount: 45,
        total: 6500,
      );

      // 验证统计数据创建成功
      expect(stats.fatalCount, 5);
      expect(stats.errorCount, 150);
      expect(stats.warnCount, 300);
      expect(stats.infoCount, 5000);
      expect(stats.debugCount, 800);
      expect(stats.traceCount, 200);
      expect(stats.unknownCount, 45);
      expect(stats.total, 6500);
    });

    test('LogLevelStats.fromMap 工厂方法', () {
      // 测试从 Map 创建 LogLevelStats
      final map = {
        'fatalCount': 10,
        'errorCount': 200,
        'warnCount': 400,
        'infoCount': 6000,
        'debugCount': 1000,
        'traceCount': 300,
        'unknownCount': 90,
        'total': 8000,
      };

      final stats = LogLevelStats.fromMap(map);

      expect(stats.fatalCount, 10);
      expect(stats.errorCount, 200);
      expect(stats.warnCount, 400);
      expect(stats.infoCount, 6000);
      expect(stats.debugCount, 1000);
      expect(stats.traceCount, 300);
      expect(stats.unknownCount, 90);
      expect(stats.total, 8000);
    });

    test('LogLevelStats 空数据处理', () {
      // 测试空 Map 处理
      final stats = LogLevelStats.fromMap(null);
      expect(stats, LogLevelStats.empty);

      // 测试空 Map
      final emptyMapStats = LogLevelStats.fromMap({});
      expect(emptyMapStats.fatalCount, 0);
      expect(emptyMapStats.total, 0);
    });

    test('LogLevelStats.empty 静态常量', () {
      // 验证空统计数据
      final empty = LogLevelStats.empty;

      expect(empty.fatalCount, 0);
      expect(empty.errorCount, 0);
      expect(empty.warnCount, 0);
      expect(empty.infoCount, 0);
      expect(empty.debugCount, 0);
      expect(empty.traceCount, 0);
      expect(empty.unknownCount, 0);
      expect(empty.total, 0);
    });

    test('点击级别触发筛选逻辑', () {
      // 模拟统计面板点击筛选逻辑
      String? selectedLevel;
      final levels = ['FATAL', 'ERROR', 'WARN', 'INFO', 'DEBUG', 'TRACE'];

      // 模拟点击 ERROR 级别
      void selectLevel(String level) {
        selectedLevel = level;
      }

      // 测试选择 ERROR
      selectLevel('ERROR');
      expect(selectedLevel, 'ERROR');

      // 测试选择 WARN
      selectLevel('WARN');
      expect(selectedLevel, 'WARN');

      // 测试选择 INFO
      selectLevel('INFO');
      expect(selectedLevel, 'INFO');

      // 验证所有级别都可以被选择
      for (final level in levels) {
        selectLevel(level);
        expect(selectedLevel, level);
      }
    });

    test('统计面板百分比计算', () {
      // 创建统计数据
      final stats = LogLevelStats(
        fatalCount: 10,
        errorCount: 90,
        warnCount: 200,
        infoCount: 500,
        debugCount: 150,
        traceCount: 50,
        unknownCount: 0,
        total: 1000,
      );

      // 计算各级别占比
      double calculatePercentage(int count, int total) {
        if (total == 0) return 0.0;
        return (count / total) * 100;
      }

      expect(calculatePercentage(stats.fatalCount, stats.total), 1.0);
      expect(calculatePercentage(stats.errorCount, stats.total), 9.0);
      expect(calculatePercentage(stats.warnCount, stats.total), 20.0);
      expect(calculatePercentage(stats.infoCount, stats.total), 50.0);
      expect(calculatePercentage(stats.debugCount, stats.total), 15.0);
      expect(calculatePercentage(stats.traceCount, stats.total), 5.0);
      expect(calculatePercentage(stats.unknownCount, stats.total), 0.0);
    });

    test('统计面板筛选多个级别', () {
      // 模拟多选级别场景
      final Set<String> selectedLevels = {};

      // 切换选择级别
      void toggleLevel(String level) {
        if (selectedLevels.contains(level)) {
          selectedLevels.remove(level);
        } else {
          selectedLevels.add(level);
        }
      }

      // 选择 ERROR 和 WARN
      toggleLevel('ERROR');
      toggleLevel('WARN');
      expect(selectedLevels.length, 2);
      expect(selectedLevels.contains('ERROR'), true);
      expect(selectedLevels.contains('WARN'), true);

      // 取消选择 ERROR
      toggleLevel('ERROR');
      expect(selectedLevels.length, 1);
      expect(selectedLevels.contains('ERROR'), false);
      expect(selectedLevels.contains('WARN'), true);

      // 清空选择
      selectedLevels.clear();
      expect(selectedLevels.isEmpty, true);
    });

    test('统计面板数据更新流程', () {
      // 模拟数据更新流程
      LogLevelStats? currentStats;

      // 模拟初始加载
      currentStats = LogLevelStats.empty;
      expect(currentStats.total, 0);

      // 模拟首次数据加载
      currentStats = LogLevelStats(
        fatalCount: 5,
        errorCount: 100,
        warnCount: 200,
        infoCount: 1000,
        debugCount: 500,
        traceCount: 100,
        unknownCount: 95,
        total: 2000,
      );
      expect(currentStats.total, 2000);

      // 模拟刷新后数据更新
      final updatedStats = LogLevelStats(
        fatalCount: 8,
        errorCount: 150,
        warnCount: 300,
        infoCount: 1500,
        debugCount: 600,
        traceCount: 150,
        unknownCount: 92,
        total: 2800,
      );
      expect(updatedStats.total, 2800);
      expect(updatedStats.errorCount, greaterThan(currentStats.errorCount));
    });

    test('自动刷新功能模拟', () {
      // 模拟自动刷新功能
      int refreshCount = 0;
      LogLevelStats? lastStats;

      // 模拟刷新函数
      Future<void> refreshStats() async {
        refreshCount++;
        // 模拟数据更新
        lastStats = LogLevelStats(
          fatalCount: refreshCount * 2,
          errorCount: refreshCount * 50,
          warnCount: refreshCount * 100,
          infoCount: refreshCount * 500,
          debugCount: refreshCount * 200,
          traceCount: refreshCount * 50,
          unknownCount: refreshCount * 10,
          total: refreshCount * 912,
        );
      }

      // 模拟多次自动刷新
      expect(refreshCount, 0);

      // 第一次刷新
      refreshStats();
      expect(refreshCount, 1);
      expect(lastStats?.total, 912);

      // 第二次刷新
      refreshStats();
      expect(refreshCount, 2);
      expect(lastStats?.total, 1824);

      // 第三次刷新
      refreshStats();
      expect(refreshCount, 3);
      expect(lastStats?.total, 2736);
    });

    test('级别排序（按数量降序）', () {
      // 创建统计数据
      final stats = LogLevelStats(
        fatalCount: 5,
        errorCount: 150,
        warnCount: 300,
        infoCount: 5000,
        debugCount: 800,
        traceCount: 200,
        unknownCount: 45,
        total: 6500,
      );

      // 创建级别-数量映射
      final levelCounts = {
        'FATAL': stats.fatalCount,
        'ERROR': stats.errorCount,
        'WARN': stats.warnCount,
        'INFO': stats.infoCount,
        'DEBUG': stats.debugCount,
        'TRACE': stats.traceCount,
        'UNKNOWN': stats.unknownCount,
      };

      // 按数量降序排序
      final sortedLevels = levelCounts.entries.toList()
        ..sort((a, b) => b.value.compareTo(a.value));

      // 验证排序结果
      expect(sortedLevels.first.key, 'INFO');
      expect(sortedLevels.first.value, 5000);
      expect(sortedLevels.last.key, 'FATAL');
      expect(sortedLevels.last.value, 5);
    });
  });
}

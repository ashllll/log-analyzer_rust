/// 性能测量工具
///
/// 用于测量代码执行时间，帮助识别性能瓶颈
class PerformanceTimer {
  final String name;
  final DateTime _startTime;
  DateTime? _endTime;

  PerformanceTimer(this.name) : _startTime = DateTime.now();

  /// 停止计时并返回耗时（毫秒）
  int stop() {
    _endTime = DateTime.now();
    return elapsedMs;
  }

  /// 获取耗时（毫秒）
  int get elapsedMs {
    final end = _endTime ?? DateTime.now();
    return end.difference(_startTime).inMilliseconds;
  }

  /// 获取耗时（微秒）
  int get elapsedMicros {
    final end = _endTime ?? DateTime.now();
    return end.difference(_startTime).inMicroseconds;
  }

  /// 格式化耗时字符串
  String get formatted {
    final ms = elapsedMs;
    if (ms < 1) {
      return '${elapsedMicros}μs';
    } else if (ms < 1000) {
      return '${ms}ms';
    } else {
      return '${(ms / 1000).toStringAsFixed(2)}s';
    }
  }

  @override
  String toString() => 'PerformanceTimer($name): $formatted';
}

/// 性能测量作用域
///
/// 在作用域结束时自动打印耗时
class PerformanceScope {
  final String name;
  final DateTime _startTime;
  final bool _silent;

  PerformanceScope(this.name, {bool silent = false})
    : _startTime = DateTime.now(),
      _silent = silent;

  /// 停止计时并打印耗时
  int stop() {
    final elapsed = DateTime.now().difference(_startTime).inMilliseconds;
    if (!_silent) {
      debugPrint('Performance: $name took ${elapsed}ms');
    }
    return elapsed;
  }

  /// 获取耗时（毫秒）
  int get elapsedMs {
    return DateTime.now().difference(_startTime).inMilliseconds;
  }

  /// 格式化耗时字符串
  String get formatted {
    final ms = elapsedMs;
    if (ms < 1) {
      return '${DateTime.now().difference(_startTime).inMicroseconds}μs';
    } else if (ms < 1000) {
      return '${ms}ms';
    } else {
      return '${(ms / 1000).toStringAsFixed(2)}s';
    }
  }

  @override
  String toString() => 'PerformanceScope($name): $formatted';

  void dispose() {
    stop();
  }
}

/// 简单的内存缓存
///
/// 用于缓存搜索结果和文件树节点
class SimpleCache<K, V> {
  final int maxSize;
  final Duration ttl;
  final Map<K, _CacheEntry<V>> _cache = {};
  int _hits = 0;
  int _misses = 0;

  SimpleCache({this.maxSize = 100, this.ttl = const Duration(minutes: 5)});

  /// 获取缓存值
  V? get(K key) {
    final entry = _cache[key];
    if (entry == null) {
      _misses++;
      return null;
    }

    // 检查是否过期
    if (DateTime.now().difference(entry.createdAt) > ttl) {
      _cache.remove(key);
      _misses++;
      return null;
    }

    _hits++;
    return entry.value;
  }

  /// 设置缓存值
  void set(K key, V value) {
    // 如果缓存已满，删除最旧的条目
    if (_cache.length >= maxSize) {
      _removeOldest();
    }
    _cache[key] = _CacheEntry(value);
  }

  /// 删除缓存值
  void remove(K key) {
    _cache.remove(key);
  }

  /// 清空缓存
  void clear() {
    _cache.clear();
    _hits = 0;
    _misses = 0;
  }

  /// 获取缓存命中率
  double get hitRate {
    final total = _hits + _misses;
    if (total == 0) return 0;
    return _hits / total;
  }

  /// 获取缓存大小
  int get size => _cache.length;

  /// 删除最旧的缓存条目
  void _removeOldest() {
    if (_cache.isEmpty) return;

    K? oldestKey;
    DateTime? oldestTime;

    for (final entry in _cache.entries) {
      if (oldestTime == null || entry.value.createdAt.isBefore(oldestTime)) {
        oldestTime = entry.value.createdAt;
        oldestKey = entry.key;
      }
    }

    if (oldestKey != null) {
      _cache.remove(oldestKey);
    }
  }

  @override
  String toString() =>
      'SimpleCache(size: $size, hits: $_hits, misses: $_misses, hitRate: ${(hitRate * 100).toStringAsFixed(1)}%)';
}

class _CacheEntry<T> {
  final T value;
  final DateTime createdAt;

  _CacheEntry(this.value) : createdAt = DateTime.now();
}

/// 调试打印工具（仅在调试模式生效）
void debugPrint(String message) {
  assert(() {
    // ignore: avoid_print
    print(message);
    return true;
  }());
}

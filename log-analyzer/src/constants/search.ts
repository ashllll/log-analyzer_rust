/**
 * SearchPage 搜索相关常量
 *
 * 集中管理魔法数字，便于调优和维护。
 *
 * 已移除（磁盘直写架构不再需要）：
 * - MAX_LOG_ENTRIES: CircularBuffer 已废弃
 * - BATCH_INTERVAL_MS: 批次定时刷新已废弃
 * - MAX_BATCH_SIZE: 批次大小阈值已废弃
 * - STREAM_SEARCH_THRESHOLD: 双模式阈值已废弃
 * - REFRESH_DEBOUNCE_MS: refreshLogs 已废弃
 */
export const SEARCH_CONFIG = {
  /** 滚动到底部时触发加载更多的像素阈值 */
  REFRESH_THRESHOLD: 50,

  /** 日志内容超过此长度时截断显示（字符数） */
  TRUNCATE_THRESHOLD: 1_000,

  /** 关键词高亮时前后显示的上下文字符数 */
  CONTEXT_LENGTH: 50,
} as const;

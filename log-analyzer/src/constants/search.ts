/**
 * SearchPage 搜索相关常量
 *
 * 集中管理魔法数字，便于调优和维护。
 */
export const SEARCH_CONFIG = {
  /** 单次搜索结果最大条目数 */
  MAX_LOG_ENTRIES: 50_000,

  /** 流式结果批量刷新间隔（ms） */
  BATCH_INTERVAL_MS: 100,

  /** 批次缓冲区满后立即刷新的阈值（条目数） */
  MAX_BATCH_SIZE: 4_000,

  /** 触发流式搜索的结果数量阈值 */
  STREAM_SEARCH_THRESHOLD: 5_000,

  /** 滚动到底部时触发加载更多的像素阈值 */
  REFRESH_THRESHOLD: 50,

  /** 防抖刷新间隔（ms） */
  REFRESH_DEBOUNCE_MS: 1_000,

  /** 日志内容超过此长度时截断显示（字符数） */
  TRUNCATE_THRESHOLD: 1_000,

  /** 关键词高亮时前后显示的上下文字符数 */
  CONTEXT_LENGTH: 50,
} as const;

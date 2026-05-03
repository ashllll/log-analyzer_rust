/**
 * 流式无限搜索 Hook
 *
 * 使用 @tanstack/react-query 的 useInfiniteQuery 实现搜索结果分页加载。
 * 新架构（磁盘直写）：搜索结果存储在后端磁盘，前端通过 fetch_search_page 按需读取。
 *
 * 特性：
 * - 虚拟滚动友好的分页加载
 * - O(1) 随机页读取（NDJSON + 二进制偏移索引）
 * - 缓存策略优化
 */

import { useInfiniteQuery, InfiniteData } from '@tanstack/react-query';
import { invoke } from '@tauri-apps/api/core';
import { LogEntry } from '../types/common';

/**
 * 后端 SearchPageResult 结构（对应 Rust search_engine::disk_result_store::SearchPageResult）
 */
interface BackendSearchPageResult {
  entries: LogEntry[];
  total_count: number;
  is_complete: boolean;
  has_more: boolean;
  next_offset: number | null;
}

/**
 * 搜索页面数据结构
 */
export interface SearchPage {
  /** 当前页结果 */
  results: LogEntry[];
  /** 下一页偏移量，null 表示没有更多数据 */
  nextOffset: number | null;
  /** 是否还有更多数据 */
  hasMore: boolean;
  /** 总条目数 */
  totalCount: number;
}

/**
 * 无限搜索配置选项
 */
export interface UseInfiniteSearchOptions {
  /** 搜索会话 ID */
  searchId: string;
  /** 搜索查询字符串 */
  query: string;
  /** 当前工作区 ID */
  workspaceId: string | null;
  /** 是否启用查询 */
  enabled: boolean;
  /** 每页大小，默认 1000 */
  pageSize?: number;
  /** 缓存时间（毫秒），默认 5 分钟 */
  staleTime?: number;
  /**
   * 内存中最大保留页数，默认 10。
   * 超出时按滑动窗口裁剪（向前加载丢弃旧页，向后加载丢弃新页）。
   * 磁盘直写架构下，被裁剪的页面可随时从磁盘重新读取。
   */
  maxPages?: number;
}


// 查询键工厂
const searchQueryKeys = {
  infinite: (searchId: string, query: string, workspaceId: string | null) =>
    ['search', 'infinite', searchId, query, workspaceId] as const,
};

/**
 * 流式无限搜索 Hook
 * 
 * @example
 * ```typescript
 * const {
 *   data,
 *   fetchNextPage,
 *   hasNextPage,
 *   isFetchingNextPage,
 * } = useInfiniteSearch({
 *   searchId: currentSearchId,
 *   query,
 *   workspaceId: activeWorkspace?.id ?? null,
 *   enabled: isSearching,
 * });
 * 
 * // 滚动到底部触发加载
 * useEffect(() => {
 *   if (isNearBottom && hasNextPage && !isFetchingNextPage) {
 *     fetchNextPage();
 *   }
 * }, [isNearBottom, hasNextPage, isFetchingNextPage]);
 * ```
 */
export function useInfiniteSearch({
  searchId,
  query,
  workspaceId,
  enabled,
  pageSize = 1000,
  staleTime = 5 * 60 * 1000, // 5 minutes
  maxPages = 10,
}: UseInfiniteSearchOptions) {
  return useInfiniteQuery<
    SearchPage,
    Error,
    InfiniteData<SearchPage>,
    ReturnType<typeof searchQueryKeys.infinite>,
    number
  >({
    queryKey: searchQueryKeys.infinite(searchId, query, workspaceId),

    queryFn: async ({ pageParam = 0 }): Promise<SearchPage> => {
      // 验证前置条件
      if (!workspaceId || !query.trim() || !searchId) {
        return {
          results: [],
          nextOffset: null,
          hasMore: false,
          totalCount: 0,
        };
      }

      try {
        // 调用后端 fetch_search_page 命令，返回包含完整元数据的 SearchPageResult
        const backendResult = await invoke<BackendSearchPageResult>('fetch_search_page', {
          searchId,
          offset: pageParam,
          limit: pageSize,
        });

        return {
          results: backendResult.entries,
          nextOffset: backendResult.next_offset,
          hasMore: backendResult.has_more,
          totalCount: backendResult.total_count,
        };
      } catch (error) {
        console.error('Failed to fetch search page:', error);
        throw error;
      }
    },

    // 获取下一页参数
    getNextPageParam: (lastPage) => lastPage.nextOffset,

    // 获取上一页参数（支持双向滑动窗口）
    getPreviousPageParam: (_firstPage, _allPages, firstPageParam) => {
      const prevOffset = (firstPageParam as number) - pageSize;
      return prevOffset >= 0 ? prevOffset : undefined;
    },

    // 初始页参数
    initialPageParam: 0,

    // 启用条件
    enabled: enabled && !!workspaceId && !!query.trim() && !!searchId,

    // 内存中最大保留页数（滑动窗口，超出时自动裁剪旧页面）
    maxPages,

    // 缓存策略
    staleTime,
    gcTime: 30 * 60 * 1000, // 30分钟，搜索结果相对稳定

    // 错误重试策略
    retry: (failureCount, error) => {
      // 如果是会话不存在错误，不重试
      if (error instanceof Error &&
          error.message.includes('not found or expired')) {
        return false;
      }
      return failureCount < 3;
    },

    // 重试延迟
    retryDelay: (attemptIndex) => Math.min(1000 * 2 ** attemptIndex, 10000),

    // 注意：不在此处使用 select 聚合 allResults。
    // 原因：flatMap 在每次数据变化时创建完整数组副本，对大结果集（10万+条）造成双重内存开销。
    // 消费者（SearchPage.tsx）已通过 loadedEntries 自行按需 flatMap。
  });
}

// 导出查询键工厂
export { searchQueryKeys };

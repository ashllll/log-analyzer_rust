/**
 * 流式无限搜索 Hook
 * 
 * 使用 @tanstack/react-query 的 useInfiniteQuery 实现流式搜索结果加载，
 * 配合后端的 VirtualSearchManager 实现分页数据获取。
 * 
 * 特性：
 * - 虚拟滚动友好的分页加载
 * - 自动内存管理 (配合 CircularBuffer)
 * - 缓存策略优化
 * - 与现有事件驱动搜索模式兼容
 */

import { useInfiniteQuery, InfiniteData } from '@tanstack/react-query';
import { invoke } from '@tauri-apps/api/core';
import { LogEntry } from '../types/common';

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
  /** 总条目数（如果已知） */
  totalCount?: number;
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
}

/**
 * 搜索结果上下文（用于与 CircularBuffer 集成）
 */
export interface SearchContext {
  /** 搜索会话 ID */
  searchId: string;
  /** 是否正在获取下一页 */
  isFetchingNextPage: boolean;
  /** 是否还有更多数据 */
  hasNextPage: boolean;
  /** 获取下一页 */
  fetchNextPage: () => Promise<void>;
  /** 获取总条目数 */
  fetchTotalCount: () => Promise<number>;
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
        // 调用后端 fetch_search_page 命令获取分页数据
        const results = await invoke<LogEntry[]>('fetch_search_page', {
          searchId,
          offset: pageParam,
          limit: pageSize,
        });

        // 判断是否还有更多数据
        const hasMore = results.length === pageSize;
        const nextOffset = hasMore ? pageParam + results.length : null;

        // 尝试获取总数（仅在第一页时）
        let totalCount: number | undefined;
        if (pageParam === 0) {
          try {
            totalCount = await invoke<number>('get_search_total_count', {
              searchId,
            });
          } catch {
            // 如果获取失败，使用当前结果估算
            totalCount = results.length;
          }
        }

        return {
          results,
          nextOffset,
          hasMore,
          totalCount,
        };
      } catch (error) {
        console.error('Failed to fetch search page:', error);
        throw error;
      }
    },
    
    // 获取下一页参数
    getNextPageParam: (lastPage) => lastPage.nextOffset,
    
    // 初始页参数
    initialPageParam: 0,
    
    // 启用条件
    enabled: enabled && !!workspaceId && !!query.trim() && !!searchId,
    
    // 缓存策略
    staleTime,
    
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
    
    // 数据选择器：聚合所有页面结果
    select: (data) => {
      // 计算总条目数
      const totalCount = data.pages[0]?.totalCount;
      
      return {
        ...data,
        totalCount,
        // 聚合所有结果（便于需要时访问）
        allResults: data.pages.flatMap(page => page.results),
      };
    },
  });
}

/**
 * 注册搜索会话到 VirtualSearchManager
 * 
 * 在搜索完成后，将结果注册到后端管理器以支持分页查询。
 * 
 * @param searchId 搜索会话 ID
 * @param query 搜索查询
 * @param entries 完整搜索结果
 * @returns 注册的 searchId
 */
export async function registerSearchSession(
  searchId: string,
  query: string,
  entries: LogEntry[]
): Promise<string> {
  return invoke<string>('register_search_session', {
    searchId,
    query,
    entries,
  });
}

/**
 * 移除搜索会话
 * 
 * 清理不再需要的搜索会话，释放内存。
 * 
 * @param searchId 搜索会话 ID
 * @returns 是否成功移除
 */
export async function removeSearchSession(searchId: string): Promise<boolean> {
  return invoke<boolean>('remove_search_session', {
    searchId,
  });
}

/**
 * 获取搜索会话信息
 * 
 * @param searchId 搜索会话 ID
 * @returns 会话信息
 */
export async function getSearchSessionInfo(
  searchId: string
): Promise<{ search_id: string; query: string; total_count: number } | null> {
  return invoke('get_search_session_info', {
    searchId,
  });
}

/**
 * 获取 VirtualSearchManager 统计信息
 * 
 * @returns 统计信息
 */
export async function getVirtualSearchStats(): Promise<{
  active_sessions: number;
  total_cached_entries: number;
  max_sessions: number;
  max_entries_per_session: number;
  session_ttl_seconds: number;
}> {
  return invoke('get_virtual_search_stats');
}

// 导出查询键工厂
export { searchQueryKeys };

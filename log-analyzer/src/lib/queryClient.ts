import { QueryClient } from '@tanstack/react-query';

/**
 * React Query 客户端配置
 * 
 * 配置说明：
 * - staleTime: 数据被认为是"新鲜"的时间（5分钟）
 * - cacheTime: 未使用的数据在缓存中保留的时间（10分钟）
 * - retry: 失败请求的重试次数
 * - refetchOnWindowFocus: 窗口重新获得焦点时自动重新获取数据
 */
export const queryClient = new QueryClient({
  defaultOptions: {
    queries: {
      staleTime: 5 * 60 * 1000, // 5 minutes
      gcTime: 10 * 60 * 1000, // 10 minutes (formerly cacheTime)
      retry: 1,
      refetchOnWindowFocus: true,
      refetchOnReconnect: true,
    },
    mutations: {
      retry: 1,
    },
  },
});

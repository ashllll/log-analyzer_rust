/**
 * 工作区时间范围 Hook
 * 封装工作区切换时自动获取时间范围的逻辑
 */
import { useState, useEffect } from 'react';
import { api } from '../../../services/api';
import type { FilterOptions } from '../../../types/common';

export interface UseWorkspaceTimeRangeOptions {
  activeWorkspaceId: string | undefined;
}

export function useWorkspaceTimeRange({ activeWorkspaceId }: UseWorkspaceTimeRangeOptions): {
  filterOptions: FilterOptions;
  setFilterOptions: React.Dispatch<React.SetStateAction<FilterOptions>>;
  resetFilters: () => void;
} {
  const [filterOptions, setFilterOptions] = useState<FilterOptions>({
    timeRange: { start: null, end: null },
    levels: [],
    filePattern: '',
  });

  useEffect(() => {
    if (!activeWorkspaceId) {
      setFilterOptions((prev) => ({
        ...prev,
        timeRange: { start: null, end: null },
      }));
      return;
    }

    let isMounted = true;

    const fetchTimeRange = async () => {
      try {
        const timeRange = await api.getWorkspaceTimeRange(activeWorkspaceId);
        if (!isMounted) return;
        if (timeRange.minTimestamp && timeRange.maxTimestamp) {
          const minDate = new Date(timeRange.minTimestamp);
          const maxDate = new Date(timeRange.maxTimestamp);

          const formatDateTimeLocal = (date: Date) => {
            const year = date.getFullYear();
            const month = String(date.getMonth() + 1).padStart(2, '0');
            const day = String(date.getDate()).padStart(2, '0');
            const hours = String(date.getHours()).padStart(2, '0');
            const minutes = String(date.getMinutes()).padStart(2, '0');
            return `${year}-${month}-${day}T${hours}:${minutes}`;
          };

          setFilterOptions((prev) => ({
            ...prev,
            timeRange: {
              start: formatDateTimeLocal(minDate),
              end: formatDateTimeLocal(maxDate),
            },
          }));
        }
      } catch {
        // 时间范围获取失败，静默处理（过滤器保持默认空值）
      }
    };

    fetchTimeRange();
    return () => {
      isMounted = false;
    };
  }, [activeWorkspaceId]);

  const resetFilters = () => {
    setFilterOptions({
      timeRange: { start: null, end: null },
      levels: [],
      filePattern: '',
    });
  };

  return { filterOptions, setFilterOptions, resetFilters };
}

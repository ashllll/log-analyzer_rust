import React, { useState, useEffect } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { Clock, X } from 'lucide-react';
import { useTranslation } from 'react-i18next';

/**
 * 搜索历史记录项
 */
export interface SearchHistoryItem {
  id: string;
  query: string;
  timestamp: number;
  result_count?: number;
  workspace_id: string;
}

interface SearchHistoryProps {
  workspaceId: string;
  onSelectQuery: (query: string) => void;
}

/**
 * 搜索历史组件
 *
 * 功能：
 * - 显示搜索历史记录列表
 * - 点击历史记录快速重用搜索
 * - 删除单条历史记录
 * - 清空所有历史
 * - 相对时间格式化（刚刚、N分钟前等）
 */
export const SearchHistory: React.FC<SearchHistoryProps> = ({
  workspaceId,
  onSelectQuery,
}) => {
  const { t } = useTranslation();
  const [history, setHistory] = useState<SearchHistoryItem[]>([]);
  const [isOpen, setIsOpen] = useState(false);
  const [isLoading, setIsLoading] = useState(false);

  // 加载历史记录
  useEffect(() => {
    loadHistory();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [workspaceId]);

  const loadHistory = async () => {
    try {
      setIsLoading(true);
      const items = await invoke<SearchHistoryItem[]>('get_search_history', {
        workspaceId,
      });
      setHistory(items);
    } catch (error) {
      console.error('Failed to load search history:', error);
    } finally {
      setIsLoading(false);
    }
  };

  const deleteItem = async (id: string) => {
    try {
      await invoke('delete_search_history', { id });
      setHistory(history.filter(item => item.id !== id));
    } catch (error) {
      console.error('Failed to delete history item:', error);
    }
  };

  const clearAll = async () => {
    try {
      await invoke('clear_search_history');
      setHistory([]);
      setIsOpen(false);
    } catch (error) {
      console.error('Failed to clear history:', error);
    }
  };

  const formatTime = (timestamp: number) => {
    const date = new Date(timestamp * 1000);
    const now = new Date();
    const diff = now.getTime() - date.getTime();

    const seconds = Math.floor(diff / 1000);
    const minutes = Math.floor(seconds / 60);
    const hours = Math.floor(minutes / 60);
    const days = Math.floor(hours / 24);

    if (seconds < 60) return '刚刚';
    if (minutes < 60) return `${minutes}分钟前`;
    if (hours < 24) return `${hours}小时前`;
    if (days < 7) return `${days}天前`;
    return date.toLocaleDateString();
  };

  return (
    <div className="relative">
      {/* 历史按钮 */}
      <button
        onClick={() => setIsOpen(!isOpen)}
        className="p-2 hover:bg-gray-700 rounded transition-colors text-gray-400 hover:text-white"
        title={t('search.searchHistory', { defaultValue: '搜索历史' })}
      >
        <Clock className="w-5 h-5" />
      </button>

      {/* 历史下拉框 */}
      {isOpen && (
        <>
          {/* 背景遮罩 */}
          <div
            className="fixed inset-0 z-10"
            onClick={() => setIsOpen(false)}
          />

          {/* 下拉框内容 */}
          <div className="absolute right-0 top-full mt-2 w-80 bg-gray-800 border border-gray-700 rounded-lg shadow-lg z-20">
            {/* 头部 */}
            <div className="flex items-center justify-between px-4 py-3 border-b border-gray-700">
              <h3 className="font-semibold text-gray-100">
                {t('search.searchHistory', { defaultValue: '搜索历史' })}
              </h3>
              <button
                onClick={clearAll}
                disabled={history.length === 0}
                className="text-sm text-gray-400 hover:text-red-400 disabled:opacity-50 disabled:cursor-not-allowed transition-colors"
              >
                {t('search.clearHistory', { defaultValue: '清空' })}
              </button>
            </div>

            {/* 列表 */}
            <div className="max-h-96 overflow-y-auto">
              {isLoading ? (
                <div className="px-4 py-8 text-center text-gray-400">
                  {t('common.loading', { defaultValue: '加载中...' })}
                </div>
              ) : history.length === 0 ? (
                <div className="px-4 py-8 text-center text-gray-400">
                  {t('search.noHistory', { defaultValue: '暂无搜索历史' })}
                </div>
              ) : (
                history.map(item => (
                  <div
                    key={item.id}
                    className="flex items-center gap-3 px-4 py-3 hover:bg-gray-700 transition-colors group"
                  >
                    <button
                      onClick={() => {
                        onSelectQuery(item.query);
                        setIsOpen(false);
                      }}
                      className="flex-1 text-left"
                    >
                      <div className="text-sm text-gray-100 font-mono">
                        {item.query}
                      </div>
                      <div className="text-xs text-gray-400 mt-1 flex items-center gap-2">
                        <span className="flex items-center gap-1">
                          <Clock className="w-3 h-3" />
                          {formatTime(item.timestamp)}
                        </span>
                        {item.result_count !== undefined && (
                          <span>
                            {item.result_count} {t('search.results', { defaultValue: '条结果' })}
                          </span>
                        )}
                      </div>
                    </button>
                    <button
                      onClick={() => deleteItem(item.id)}
                      className="p-1 text-gray-500 hover:text-red-400 opacity-0 group-hover:opacity-100 transition-all"
                      title={t('search.deleteHistory', { defaultValue: '删除' })}
                    >
                      <X className="w-4 h-4" />
                    </button>
                  </div>
                ))
              )}
            </div>

            {/* 底部 */}
            {history.length > 0 && (
              <div className="px-4 py-2 border-t border-gray-700 text-xs text-gray-500">
                {t('search.historyCount', { defaultValue: '共 {{count}} 条', count: history.length })}
              </div>
            )}
          </div>
        </>
      )}
    </div>
  );
};

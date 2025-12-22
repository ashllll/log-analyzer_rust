import { useCallback } from 'react';
import { useAppStore } from '../stores/appStore';
import { useKeywordStore, KeywordGroup } from '../stores/keywordStore';

/**
 * 关键词管理Hook
 * 封装关键词组的CRUD操作
 */
export const useKeywordManager = () => {
  const addToast = useAppStore((state) => state.addToast);
  const keywordGroups = useKeywordStore((state) => state.keywordGroups);
  const keywordsLoading = useKeywordStore((state) => state.loading);
  const keywordsError = useKeywordStore((state) => state.error);
  const addKeywordGroupAction = useKeywordStore((state) => state.addKeywordGroup);
  const updateKeywordGroupAction = useKeywordStore((state) => state.updateKeywordGroup);
  const deleteKeywordGroupAction = useKeywordStore((state) => state.deleteKeywordGroup);
  const toggleKeywordGroupAction = useKeywordStore((state) => state.toggleKeywordGroup);

  /**
   * 添加关键词组
   */
  const addKeywordGroup = useCallback((group: KeywordGroup) => {
    addKeywordGroupAction(group);
    addToast('success', '关键词组已创建');
  }, [addToast, addKeywordGroupAction]);

  /**
   * 更新关键词组
   */
  const updateKeywordGroup = useCallback((group: KeywordGroup) => {
    updateKeywordGroupAction(group);
    addToast('success', '关键词组已更新');
  }, [addToast, updateKeywordGroupAction]);

  /**
   * 删除关键词组
   */
  const deleteKeywordGroup = useCallback((id: string) => {
    deleteKeywordGroupAction(id);
    addToast('info', '关键词组已删除');
  }, [addToast, deleteKeywordGroupAction]);

  /**
   * 切换关键词组启用状态
   */
  const toggleKeywordGroup = useCallback((id: string) => {
    toggleKeywordGroupAction(id);
  }, [toggleKeywordGroupAction]);

  /**
   * 保存关键词组（新增或更新）
   */
  const saveKeywordGroup = useCallback((group: KeywordGroup, isEditing: boolean) => {
    if (isEditing) {
      updateKeywordGroup(group);
    } else {
      addKeywordGroup(group);
    }
  }, [addKeywordGroup, updateKeywordGroup]);

  return {
    keywordGroups,
    loading: keywordsLoading,
    error: keywordsError,
    addKeywordGroup,
    updateKeywordGroup,
    deleteKeywordGroup,
    toggleKeywordGroup,
    saveKeywordGroup
  };
};

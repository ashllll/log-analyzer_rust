import { useCallback } from 'react';
import { useApp, useKeywordState, KeywordGroup } from '../contexts/AppContext';

/**
 * 关键词管理Hook
 * 封装关键词组的CRUD操作
 */
export const useKeywordManager = () => {
  const { addToast } = useApp();
  const { state: keywordState, dispatch: keywordDispatch } = useKeywordState();

  /**
   * 添加关键词组
   */
  const addKeywordGroup = useCallback((group: KeywordGroup) => {
    keywordDispatch({ type: 'ADD_KEYWORD_GROUP', payload: group });
    addToast('success', '关键词组已创建');
  }, [addToast, keywordDispatch]);

  /**
   * 更新关键词组
   */
  const updateKeywordGroup = useCallback((group: KeywordGroup) => {
    keywordDispatch({ type: 'UPDATE_KEYWORD_GROUP', payload: group });
    addToast('success', '关键词组已更新');
  }, [addToast, keywordDispatch]);

  /**
   * 删除关键词组
   */
  const deleteKeywordGroup = useCallback((id: string) => {
    keywordDispatch({ type: 'DELETE_KEYWORD_GROUP', payload: id });
    addToast('info', '关键词组已删除');
  }, [addToast, keywordDispatch]);

  /**
   * 切换关键词组启用状态
   */
  const toggleKeywordGroup = useCallback((id: string) => {
    keywordDispatch({ type: 'TOGGLE_KEYWORD_GROUP', payload: id });
  }, [keywordDispatch]);

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
    keywordGroups: keywordState.keywordGroups,
    loading: keywordState.loading,
    error: keywordState.error,
    addKeywordGroup,
    updateKeywordGroup,
    deleteKeywordGroup,
    toggleKeywordGroup,
    saveKeywordGroup
  };
};

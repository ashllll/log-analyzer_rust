import { useEffect, useRef } from 'react';
import { useWorkspaceStore } from '../stores/workspaceStore';
import { useKeywordStore } from '../stores/keywordStore';
import { useAppStore } from '../stores/appStore';
import { useToast } from './useToast';
import { useConfigQuery } from './useServerQueries';
import type { Workspace } from '../types/common';
import { logger } from '../utils/logger';

/**
 * 配置初始化 Hook
 *
 * 负责在应用启动时从后端加载配置（workspaces、keyword_groups），
 * 并标记应用为已初始化状态。加载失败时提供默认工作区作为降级方案。
 */
export const useConfigInitializer = () => {
  const { showToast: addToast } = useToast();
  const setWorkspaces = useWorkspaceStore((state) => state.setWorkspaces);
  const setKeywordGroups = useKeywordStore((state) => state.setKeywordGroups);
  const setInitPhase = useAppStore((state) => state.setInitPhase);
  const didHandleErrorRef = useRef(false);
  const query = useConfigQuery();

  useEffect(() => {
    if (query.isLoading || query.isFetching) {
      setInitPhase('loading');
    }
  }, [query.isFetching, query.isLoading, setInitPhase]);

  useEffect(() => {
    if (!query.isSuccess) {
      return;
    }

    didHandleErrorRef.current = false;
    setInitPhase('ready');
  }, [query.isSuccess, setInitPhase]);

  useEffect(() => {
    if (!query.isError || didHandleErrorRef.current) {
      return;
    }

    didHandleErrorRef.current = true;
    logger.error({ error: query.error }, 'Failed to load config');
    // 配置加载失败时提供默认工作区作为降级方案
    const defaultWorkspace: Workspace = {
      id: 'default-workspace',
      name: '默认工作区',
      path: '',
      status: 'OFFLINE',
      size: '0 B',
      files: 0,
      watching: false,
    };
    setWorkspaces([defaultWorkspace]);
    setKeywordGroups([]);
    addToast('error', '加载配置失败，使用默认工作区');
    setInitPhase('error');
  }, [
    addToast,
    query.error,
    query.isError,
    setWorkspaces,
    setKeywordGroups,
    setInitPhase,
  ]);
};

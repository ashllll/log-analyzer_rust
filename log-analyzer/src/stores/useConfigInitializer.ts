import { useEffect } from 'react';
import { useWorkspaceStore } from './workspaceStore';
import { useKeywordStore } from './keywordStore';
import { useAppStore } from './appStore';
import { useToast } from '../hooks/useToast';
import { api } from '../services/api';
import type { Workspace, KeywordGroup } from '../types/common';
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
  const setInitialized = useAppStore((state) => state.setInitialized);

  useEffect(() => {
    let isMounted = true;

    const loadConfig = async () => {
      try {
        const config = await api.loadConfig();
        if (!isMounted) return;

        setWorkspaces(Array.isArray(config.workspaces) ? (config.workspaces as Workspace[]) : []);
        setKeywordGroups(
          Array.isArray(config.keyword_groups) ? (config.keyword_groups as KeywordGroup[]) : []
        );

        setInitialized(true);
      } catch (error) {
        if (!isMounted) return;
        logger.error({ error }, 'Failed to load config');
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
        setInitialized(true); // 关键：即使失败也标记为已初始化
      }
    };

    // 延迟加载配置，避免阻塞首屏渲染。
    // 不要用 ref 短路 effect；React StrictMode 会在开发态额外执行 setup+cleanup，
    // ref 短路会导致第一次 timer 被 cleanup 清掉后，第二次 setup 不再重新初始化。
    const timer = window.setTimeout(() => {
      loadConfig();
    }, 100);

    return () => {
      isMounted = false;
      window.clearTimeout(timer);
    };
  }, [addToast, setWorkspaces, setKeywordGroups, setInitialized]);
};

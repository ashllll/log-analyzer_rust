import { useEffect, useRef } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { useWorkspaceStore } from './workspaceStore';
import { useKeywordStore } from './keywordStore';
import { useAppStore } from './appStore';
import { useToast } from '../hooks/useToast';
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

  // 使用 ref 跟踪初始化状态，防止 React StrictMode 重复初始化
  const initializedRef = useRef(false);

  useEffect(() => {
    if (initializedRef.current) return;
    initializedRef.current = true;

    let isMounted = true;

    const loadConfig = async () => {
      try {
        const config = await invoke<Record<string, unknown>>('load_config');
        if (!isMounted) return;

        if (config) {
          if (config.workspaces) {
            setWorkspaces(config.workspaces as Workspace[]);
          }
          if (config.keyword_groups) {
            setKeywordGroups(config.keyword_groups as KeywordGroup[]);
          }
        }

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

    // 延迟加载配置，避免阻塞首屏渲染
    const timer = setTimeout(() => {
      loadConfig();
    }, 100);

    return () => {
      isMounted = false;
      clearTimeout(timer);
    };
  }, [addToast, setWorkspaces, setKeywordGroups, setInitialized]);
};
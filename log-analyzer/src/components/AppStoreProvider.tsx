import { ReactNode } from 'react';
import { useConfigManager } from '../hooks/useConfigManager';
import { useConfigInitializer } from '../hooks/useConfigInitializer';
import { useEventBusSubscriptions } from '../hooks/useEventBusSubscriptions';
import { useTauriEventListeners } from '../hooks/useTauriEventListeners';

interface AppStoreProviderProps {
  children: ReactNode;
}

/**
 * AppStoreProvider - 初始化 zustand stores 并设置事件监听器
 *
 * 这个组件负责：
 * 1. 在应用启动时加载配置
 * 2. 设置后端事件监听器
 * 3. 在组件卸载时清理事件监听器
 *
 * ## 任务生命周期管理
 *
 * 采用业内成熟的事件驱动架构：
 * - task-update: 任务状态更新（创建、进度、完成）
 * - task-removed: 任务自动清理（后端 Actor 触发）
 * - import-complete: 导入完成事件
 * - import-error: 导入错误事件
 */
export const AppStoreProvider = ({ children }: AppStoreProviderProps) => {
  // 启用配置自动保存（防抖1000ms后保存到后端）
  useConfigManager();

  // 配置加载：从后端加载 workspaces 和 keyword_groups
  useConfigInitializer();

  // EventBus 订阅：处理 task-update、task-removed 等应用层事件
  useEventBusSubscriptions();

  // Tauri 原生事件监听：桥接后端 IPC 事件到 EventBus
  useTauriEventListeners();

  return <>{children}</>;
};

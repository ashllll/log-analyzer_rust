import { useEffect } from 'react';
import { logger } from '../utils/logger';

/**
 * EventManager component - DEPRECATED
 * 
 * 所有事件监听已迁移到 AppStoreProvider：
 * - task-update: 由 AppStoreProvider 统一处理（带 Schema 验证 + 幂等性检查）
 * - task-removed: 由 AppStoreProvider 统一处理
 * - import-complete: 由 AppStoreProvider 统一处理
 * - import-error: 由 AppStoreProvider 统一处理
 * 
 * 保留此组件是为了向后兼容，避免破坏可能导入此组件的其他代码。
 * 此组件不再监听任何事件。
 * 
 * @deprecated 所有事件处理已迁移到 AppStoreProvider
 */
export const EventManager = () => {
  useEffect(() => {
    logger.debug('[EVENT_MANAGER] EventManager is deprecated. All event handling has been migrated to AppStoreProvider.');

    // 此组件不再监听任何事件
    // 所有事件监听已由 AppStoreProvider 接管

    return () => {
      logger.debug('[EVENT_MANAGER] Cleanup (no-op)');
    };
  }, []);

  // This component doesn't render anything
  return null;
};

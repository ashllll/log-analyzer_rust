/**
 * IPC 连接预热机制
 * 
 * 业内成熟方案：
 * 1. 应用启动时预热 IPC 连接
 * 2. 预加载常用命令
 * 3. 建立连接池
 * 4. 保持连接活跃
 */

import { invoke } from '@tauri-apps/api/core';
import { logger } from './logger';
import { getIPCHealthChecker } from './ipcHealthCheck';

export interface WarmupResult {
  success: boolean;
  duration: number;
  errors: string[];
}

/**
 * IPC 连接预热
 * 在应用启动时调用，建立稳定的 IPC 连接
 */
export async function warmupIPCConnection(): Promise<WarmupResult> {
  const startTime = Date.now();
  const errors: string[] = [];
  
  logger.debug('[IPCWarmup] Starting IPC connection warmup...');
  
  try {
    // 1. 预热轻量级命令（建立连接）
    const warmupCommands = [
      { name: 'load_config', args: {} },
      { name: 'check_rar_support', args: {} },
    ];
    
    for (const cmd of warmupCommands) {
      try {
        await invoke(cmd.name, cmd.args);
        logger.debug(`[IPCWarmup] Warmed up command: ${cmd.name}`);
      } catch (error) {
        const errorMsg = `Failed to warmup ${cmd.name}: ${error}`;
        logger.warn(`[IPCWarmup] ${errorMsg}`);
        errors.push(errorMsg);
      }
    }
    
    // 2. 启动健康检查
    const healthChecker = getIPCHealthChecker();
    const isHealthy = await healthChecker.checkNow();
    
    if (!isHealthy) {
      errors.push('IPC health check failed after warmup');
      logger.error('[IPCWarmup] IPC connection unhealthy after warmup');
    } else {
      logger.debug('[IPCWarmup] IPC health check passed');
    }
    
    const duration = Date.now() - startTime;
    const success = errors.length === 0;
    
    if (success) {
      logger.debug(`[IPCWarmup] Warmup completed successfully in ${duration}ms`);
    } else {
      logger.warn(`[IPCWarmup] Warmup completed with ${errors.length} errors in ${duration}ms`);
    }
    
    return {
      success,
      duration,
      errors,
    };
  } catch (error) {
    const duration = Date.now() - startTime;
    const errorMsg = `Warmup failed: ${error}`;
    logger.error(`[IPCWarmup] ${errorMsg}`);
    
    return {
      success: false,
      duration,
      errors: [errorMsg, ...errors],
    };
  }
}

/**
 * 在应用启动时自动预热
 * 应该在 App 组件的 useEffect 中调用
 */
export function initializeIPCConnection(): void {
  // 延迟预热，避免阻塞应用启动
  setTimeout(async () => {
    const result = await warmupIPCConnection();
    
    if (!result.success) {
      logger.error('[IPCWarmup] Failed to initialize IPC connection:', result.errors);
    }
  }, 100);
}

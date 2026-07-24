/**
 * 错误日志管理器
 *
 * 职责：
 * - 错误日志持久化到 localStorage
 * - 错误报告器配置
 * - 日志查询、清除、导出
 */

// ============================================================================
// 类型定义
// ============================================================================

export interface ErrorLogEntry {
  timestamp: string;
  message: string;
  stack?: string;
  componentStack?: string;
  type: "error" | "unhandled_rejection" | "global_error";
}

export interface ErrorReportContext {
  source: ErrorLogEntry["type"];
  componentStack?: string;
}

export interface ErrorReporter {
  captureException(error: unknown, context: ErrorReportContext): void;
}

// ============================================================================
// 错误报告器配置
// ============================================================================

let errorReporter: ErrorReporter | undefined;

export function configureErrorReporter(
  reporter: ErrorReporter | undefined
): void {
  errorReporter = reporter;
}

export function reportError(error: unknown, context: ErrorReportContext): void {
  try {
    errorReporter?.captureException(error, context);
  } catch {
    // Silently ignore reporter failures to avoid infinite error loops
  }
}

// ============================================================================
// 错误日志存储
// ============================================================================

const ERROR_LOG_CONFIG = {
  /** 存储键名 */
  STORAGE_KEY: "app_error_logs",
  /** 最大保存条目数 */
  MAX_ENTRIES: 100,
  /** 日志保留天数 */
  RETENTION_DAYS: 7,
};

/**
 * 获取所有错误日志
 */
export function getErrorLogs(): ErrorLogEntry[] {
  try {
    const stored = localStorage.getItem(ERROR_LOG_CONFIG.STORAGE_KEY);
    if (stored) {
      const logs = JSON.parse(stored) as ErrorLogEntry[];
      // 过滤过期日志
      const cutoffDate = new Date();
      cutoffDate.setDate(
        cutoffDate.getDate() - ERROR_LOG_CONFIG.RETENTION_DAYS
      );
      return logs.filter((log) => new Date(log.timestamp) > cutoffDate);
    }
  } catch {
    // localStorage 不可用或数据损坏，静默返回空数组
  }
  return [];
}

/**
 * 添加错误日志
 */
export function addErrorLog(entry: ErrorLogEntry): void {
  try {
    const logs = getErrorLogs();
    logs.unshift(entry); // 添加到开头
    // 限制条目数量
    const trimmedLogs = logs.slice(0, ERROR_LOG_CONFIG.MAX_ENTRIES);
    localStorage.setItem(
      ERROR_LOG_CONFIG.STORAGE_KEY,
      JSON.stringify(trimmedLogs)
    );
  } catch {
    // localStorage 写入失败，静默忽略
  }
}

/**
 * 清除所有错误日志
 */
export function clearErrorLogs(): void {
  try {
    localStorage.removeItem(ERROR_LOG_CONFIG.STORAGE_KEY);
  } catch {
    // localStorage 清除失败，静默忽略
  }
}

/**
 * 导出错误日志（用于报告问题）
 */
export function exportErrorLogs(): string {
  const logs = getErrorLogs();
  try {
    return JSON.stringify(logs, null, 2);
  } catch {
    // 处理循环引用或不可序列化的对象
    return JSON.stringify(
      logs.map((l) => ({
        ...l,
        stack: l.stack ? String(l.stack) : undefined,
      })),
      null,
      2
    );
  }
}

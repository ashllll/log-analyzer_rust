// 统一日志工具
/**
 * 结构化日志工具
 *
 * 特性：
 * 1. 开发模式：彩色、详细日志
 * 2. 生产模式：JSON格式、可控级别
 * 3. 零依赖：纯TypeScript实现
 *
 * @module utils/logger
 * @author Claude (老王)
 * @created 2025-12-27
 */

type LogLevel = 'debug' | 'info' | 'warn' | 'error';

interface LogContext {
  [key: string]: any;
}

/**
 * 日志级别
 */
export enum LogLevelEnum {
  DEBUG = 'DEBUG',
  INFO = 'INFO',
  WARN = 'WARN',
  ERROR = 'ERROR'
}

/**
 * Logger类
 */
class LoggerImpl {
  private isDev = import.meta.env.DEV;
  private logLevel: LogLevel = 'info';

  private shouldLog(level: LogLevel): boolean {
    const levels: LogLevel[] = ['debug', 'info', 'warn', 'error'];
    return levels.indexOf(level) >= levels.indexOf(this.logLevel);
  }

  private format(level: LogLevel, context: LogContext, message?: string): string {
    const timestamp = new Date().toISOString();

    if (this.isDev && context && Object.keys(context).length > 0) {
      // 开发模式：结构化、易读格式
      const colors = {
        debug: '\x1b[36m', // cyan
        info: '\x1b[32m',  // green
        warn: '\x1b[33m',  // yellow
        error: '\x1b[31m', // red
      };
      const reset = '\x1b[0m';

      const prefix = `${colors[level]}[${level.toUpperCase()}]${reset} [${timestamp}]`;

      if (message) {
        return `${prefix} ${message} ${JSON.stringify(context, null, 2)}`;
      }
      return `${prefix} ${JSON.stringify(context, null, 2)}`;
    } else {
      // 简单模式：兼容原有格式
      if (message && (!context || Object.keys(context).length === 0)) {
        return `[${level.toUpperCase()}] ${message}`;
      }
      return `[${level.toUpperCase()}] ${message || ''} ${JSON.stringify(context)}`;
    }
  }

  /**
   * 调试日志(仅在开发环境输出)
   */
  debug(message: string, ...args: any[]): void;
  debug(context: LogContext, message?: string): void;

  debug(messageOrContext: string | LogContext, ...args: any[]): void {
    if (typeof messageOrContext === 'string') {
      // 兼容原有API
      if (this.shouldLog('debug')) {
        console.log(`[${LogLevelEnum.DEBUG}] ${messageOrContext}`, ...args);
      }
    } else {
      // 新的结构化API
      if (this.shouldLog('debug')) {
        console.log(this.format('debug', messageOrContext as LogContext));
      }
    }
  }

  /**
   * 信息日志
   */
  info(message: string, ...args: any[]): void;
  info(context: LogContext, message?: string): void;

  info(messageOrContext: string | LogContext, ...args: any[]): void {
    if (typeof messageOrContext === 'string') {
      // 兼容原有API
      console.log(`[${LogLevelEnum.INFO}] ${messageOrContext}`, ...args);
    } else {
      // 新的结构化API
      console.log(this.format('info', messageOrContext as LogContext));
    }
  }

  /**
   * 警告日志
   */
  warn(message: string, ...args: any[]): void;
  warn(context: LogContext, message?: string): void;

  warn(messageOrContext: string | LogContext, ...args: any[]): void {
    if (typeof messageOrContext === 'string') {
      // 兼容原有API
      console.warn(`[WARN] ${messageOrContext}`, ...args);
    } else {
      // 新的结构化API
      console.warn(this.format('warn', messageOrContext as LogContext));
    }
  }

  /**
   * 错误日志
   */
  error(message: string, ...args: any[]): void;
  error(context: LogContext, message?: string): void;

  error(messageOrContext: string | LogContext, ...args: any[]): void {
    if (typeof messageOrContext === 'string') {
      // 兼容原有API
      console.error(`[${LogLevelEnum.ERROR}] ${messageOrContext}`, ...args);
    } else {
      // 新的结构化API
      console.error(this.format('error', messageOrContext as LogContext));
    }
  }

  /**
   * 设置日志级别
   */
  setLevel(level: LogLevel): void {
    this.logLevel = level;
  }

  /**
   * 获取当前日志级别
   */
  getLevel(): LogLevel {
    return this.logLevel;
  }
}

/**
 * 全局Logger实例
 */
export const logger = new LoggerImpl();

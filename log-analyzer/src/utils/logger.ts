// 统一日志工具

/**
 * 日志级别
 */
export enum LogLevel {
  DEBUG = 'DEBUG',
  INFO = 'INFO',
  ERROR = 'ERROR'
}

/**
 * 日志工具对象
 */
export const logger = {
  /**
   * 调试日志(仅在开发环境输出)
   */
  debug: (message: string, ...args: any[]) => {
    if (import.meta.env.DEV) {
      console.log(`[${LogLevel.DEBUG}] ${message}`, ...args);
    }
  },
  
  /**
   * 信息日志
   */
  info: (message: string, ...args: any[]) => {
    console.log(`[${LogLevel.INFO}] ${message}`, ...args);
  },
  
  /**
   * 错误日志
   */
  error: (message: string, ...args: any[]) => {
    console.error(`[${LogLevel.ERROR}] ${message}`, ...args);
  }
};

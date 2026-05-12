/**
 * 错误处理模块
 *
 * 定义错误码、错误类型和错误处理工具函数
 * 支持i18n国际化错误消息
 *
 * @module errors
 */

import i18n from '../i18n';

// ============================================================================
// 错误码定义
// ============================================================================

/**
 * 错误码枚举 - 与后端 AppError 对应
 * 用于错误分类和 i18n 消息映射
 */
export enum ErrorCode {
  IO_ERROR = 'IO_ERROR',
  SEARCH_ERROR = 'SEARCH_ERROR',
  ARCHIVE_ERROR = 'ARCHIVE_ERROR',
  VALIDATION_ERROR = 'VALIDATION_ERROR',
  SECURITY_ERROR = 'SECURITY_ERROR',
  NOT_FOUND = 'NOT_FOUND',
  INVALID_PATH = 'INVALID_PATH',
  ENCODING_ERROR = 'ENCODING_ERROR',
  QUERY_EXECUTION_ERROR = 'QUERY_EXECUTION_ERROR',
  FILE_WATCHER_ERROR = 'FILE_WATCHER_ERROR',
  INDEX_ERROR = 'INDEX_ERROR',
  PATTERN_ERROR = 'PATTERN_ERROR',
  DATABASE_ERROR = 'DATABASE_ERROR',
  CONFIG_ERROR = 'CONFIG_ERROR',
  NETWORK_ERROR = 'NETWORK_ERROR',
  INTERNAL_ERROR = 'INTERNAL_ERROR',
  RESOURCE_CLEANUP_ERROR = 'RESOURCE_CLEANUP_ERROR',
  CONCURRENCY_ERROR = 'CONCURRENCY_ERROR',
  PARSE_ERROR = 'PARSE_ERROR',
  TIMEOUT_ERROR = 'TIMEOUT_ERROR',
  UNKNOWN = 'UNKNOWN',
}

// ============================================================================
// 错误类型定义
// ============================================================================

/**
 * 结构化错误接口
 *
 * 与后端 CommandError 对应
 */
export interface StructuredError {
  /** 错误码 */
  code: string;

  /** 用户友好的错误消息 */
  message: string;

  /** 帮助提示 (可选) */
  help?: string;

  /** 错误详情 (可选，用于调试) */
  details?: unknown;
}

/**
 * 错误分类
 */
export enum ErrorCategory {
  /** 用户输入错误 */
  USER = 'user',

  /** 系统错误 */
  SYSTEM = 'system',

  /** 网络错误 */
  NETWORK = 'network',

  /** 文件系统错误 */
  FILESYSTEM = 'filesystem',

  /** 未知错误 */
  UNKNOWN = 'unknown',
}

// ============================================================================
// 错误码到分类的映射
// ============================================================================

const ERROR_CODE_CATEGORIES: Record<string, ErrorCategory> = {
  [ErrorCode.VALIDATION_ERROR]: ErrorCategory.USER,
  [ErrorCode.PATTERN_ERROR]: ErrorCategory.USER,
  [ErrorCode.INVALID_PATH]: ErrorCategory.USER,
  [ErrorCode.SECURITY_ERROR]: ErrorCategory.USER,

  [ErrorCode.IO_ERROR]: ErrorCategory.FILESYSTEM,
  [ErrorCode.ENCODING_ERROR]: ErrorCategory.FILESYSTEM,
  [ErrorCode.FILE_WATCHER_ERROR]: ErrorCategory.FILESYSTEM,

  [ErrorCode.NETWORK_ERROR]: ErrorCategory.NETWORK,

  [ErrorCode.DATABASE_ERROR]: ErrorCategory.SYSTEM,
  [ErrorCode.CONCURRENCY_ERROR]: ErrorCategory.SYSTEM,
  [ErrorCode.RESOURCE_CLEANUP_ERROR]: ErrorCategory.SYSTEM,
  [ErrorCode.INTERNAL_ERROR]: ErrorCategory.SYSTEM,
};

// ============================================================================
// 错误码到i18n键的映射
// ============================================================================

const ERROR_CODE_I18N_KEYS: Record<string, string> = {
  [ErrorCode.IO_ERROR]: 'errors.io.error',
  [ErrorCode.SEARCH_ERROR]: 'errors.search.execution_error',
  [ErrorCode.ARCHIVE_ERROR]: 'errors.io.error',
  [ErrorCode.VALIDATION_ERROR]: 'errors.validation.path_canonicalization_failed',
  [ErrorCode.SECURITY_ERROR]: 'errors.validation.path_traversal',
  [ErrorCode.NOT_FOUND]: 'errors.workspace.not_found',
  [ErrorCode.INVALID_PATH]: 'errors.validation.path_canonicalization_failed',
  [ErrorCode.ENCODING_ERROR]: 'errors.io.encoding',
  [ErrorCode.QUERY_EXECUTION_ERROR]: 'errors.search.execution_error',
  [ErrorCode.FILE_WATCHER_ERROR]: 'errors.watch.watcher_create_failed',
  [ErrorCode.INDEX_ERROR]: 'errors.search.database_error',
  [ErrorCode.PATTERN_ERROR]: 'errors.keywords.invalid_regex',
  [ErrorCode.DATABASE_ERROR]: 'errors.search.database_error',
  [ErrorCode.CONFIG_ERROR]: 'errors.config.validation_failed',
  [ErrorCode.NETWORK_ERROR]: 'errors.io.error',
  [ErrorCode.INTERNAL_ERROR]: 'errors.unknown',
  [ErrorCode.RESOURCE_CLEANUP_ERROR]: 'errors.resource.cleanup_failed',
  [ErrorCode.CONCURRENCY_ERROR]: 'errors.task.operation_timeout',
  [ErrorCode.PARSE_ERROR]: 'errors.io.error',
  [ErrorCode.TIMEOUT_ERROR]: 'errors.search.timeout',
  [ErrorCode.UNKNOWN]: 'errors.unknown',
};

/**
 * 获取本地化的错误消息
 *
 * @param code - 错误码
 * @param params - 可选的参数对象
 * @returns 本地化的错误消息
 */
export function getLocalizedErrorMessage(code: string, params?: Record<string, unknown>): string {
  const key = ERROR_CODE_I18N_KEYS[code] || 'errors.unknown';
  return i18n.t(key, params || {});
}

// ============================================================================
// API 错误类
// ============================================================================

/** 封装 Tauri 命令调用错误，提供结构化访问 */
export class ApiError extends Error implements StructuredError {
  code: string;
  help?: string;
  details?: unknown;
  cause?: unknown;

  constructor(
    public command: string,
    message: string,
    cause?: unknown
  ) {
    super(message);
    this.name = 'ApiError';
    this.code = ErrorCode.UNKNOWN;
    this.cause = cause;
    this.parseStructuredError(cause);
  }

  private parseStructuredError(cause?: unknown): void {
    if (!cause) return;

    // 如果是字符串，尝试解析 JSON
    if (typeof cause === 'string') {
      try {
        const parsed = JSON.parse(cause) as StructuredError;
        this.code = parsed.code || this.code;
        this.message = parsed.message || this.message;
        this.help = parsed.help;
        this.details = parsed.details;
      } catch {
        // 不是 JSON，保持默认值
      }
      return;
    }

    // 如果是对象，尝试提取结构化信息
    if (typeof cause === 'object' && cause !== null) {
      const err = cause as Record<string, unknown>;
      this.code = (err.code as string) || this.code;
      this.help = err.help as string;
      this.details = err.details;
    }
  }

  getCategory(): ErrorCategory {
    return ERROR_CODE_CATEGORIES[this.code] || ErrorCategory.UNKNOWN;
  }

  isErrorCode(code: ErrorCode): boolean {
    return this.code === code;
  }

  isUserError(): boolean {
    return this.getCategory() === ErrorCategory.USER;
  }

  isSystemError(): boolean {
    return this.getCategory() === ErrorCategory.SYSTEM;
  }

  isNetworkError(): boolean {
    return this.getCategory() === ErrorCategory.NETWORK;
  }

  isFilesystemError(): boolean {
    return this.getCategory() === ErrorCategory.FILESYSTEM;
  }

  /** 优先保留原始消息，未结构化错误再回退到 i18n 文案 */
  getUserMessage(): string {
    if (this.message && this.message !== this.code) {
      return this.message;
    }
    const localizedMessage = getLocalizedErrorMessage(this.code);
    if (localizedMessage && localizedMessage !== 'errors.unknown') {
      return localizedMessage;
    }
    return this.message;
  }

  getLocalizedMessage(): string {
    return getLocalizedErrorMessage(this.code);
  }

  getFullMessage(): string {
    const userMessage = this.getUserMessage();
    if (this.help) {
      const helpLabel = i18n.t('errors.details');
      return `${userMessage}\n${helpLabel}：${this.help}`;
    }
    return userMessage;
  }

  toJSON(): StructuredError {
    return {
      code: this.code,
      message: this.message,
      help: this.help,
      details: this.details,
    };
  }
}

// ============================================================================
// 错误处理工具函数
// ============================================================================

/** 从未知错误创建 ApiError */
export function createApiError(command: string, error: unknown): ApiError {
  if (error instanceof ApiError) {
    return error;
  }
  const message = error instanceof Error ? error.message : String(error);
  return new ApiError(command, message, error);
}

/** 判断错误是否可重试（超时/网络/并发） */
export function isRetryableError(error: unknown): boolean {
  if (error instanceof ApiError) {
    return (
      error.isErrorCode(ErrorCode.TIMEOUT_ERROR) ||
      error.isErrorCode(ErrorCode.NETWORK_ERROR) ||
      error.isErrorCode(ErrorCode.CONCURRENCY_ERROR)
    );
  }
  return false;
}

/** 获取错误消息用于显示 */
export function getErrorMessage(error: unknown): string {
  if (error instanceof ApiError) {
    return error.getUserMessage();
  }
  if (error instanceof Error) {
    return error.message;
  }
  return String(error);
}

/** 获取完整错误提示（包含帮助信息） */
export function getFullErrorMessage(error: unknown): string {
  if (error instanceof ApiError) {
    return error.getFullMessage();
  }
  return getErrorMessage(error);
}

/** 错误处理装饰器 - 自动将异常转换为 ApiError */
export function withErrorHandler<T extends (...args: unknown[]) => Promise<unknown>>(
  command: string,
  fn: T
): T {
  return (async (...args: unknown[]) => {
    try {
      return await fn(...args);
    } catch (error) {
      throw createApiError(command, error);
    }
  }) as T;
}

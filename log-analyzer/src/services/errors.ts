/**
 * 错误处理模块
 *
 * 定义错误码、错误类型和错误处理工具函数
 *
 * @module errors
 */

// ============================================================================
// 错误码定义
// ============================================================================

/**
 * 错误码枚举
 *
 * 定义所有可能的错误码，与后端 AppError 对应
 *
 * @description
 * 错误码用于标识错误的类型，帮助前端进行错误分类和处理。
 * 每个错误码都对应一个特定的错误场景，并提供默认的错误消息。
 *
 * @example
 * ```typescript
 * if (error instanceof ApiError && error.isErrorCode(ErrorCode.NOT_FOUND)) {
 *   // 处理未找到错误
 * }
 * ```
 */
export enum ErrorCode {
  // IO 错误
  /**
   * IO 错误
   *
   * 文件读写失败、磁盘空间不足、权限问题等
   *
   * @example
   * ```typescript
   * if (error.isErrorCode(ErrorCode.IO_ERROR)) {
   *   showMessage('文件操作失败，请检查磁盘空间和权限');
   * }
   * ```
   */
  IO_ERROR = 'IO_ERROR',

  // 搜索错误
  /**
   * 搜索错误
   *
   * 搜索查询失败、索引损坏、工作区不可用等
   *
   * @example
   * ```typescript
   * if (error.isErrorCode(ErrorCode.SEARCH_ERROR)) {
   *   showMessage('搜索失败，请尝试简化搜索词或重新加载工作区');
   * }
   * ```
   */
  SEARCH_ERROR = 'SEARCH_ERROR',

  // 归档错误
  /**
   * 归档错误
   *
   * 压缩包处理失败、格式不支持、文件损坏等
   *
   * @example
   * ```typescript
   * if (error.isErrorCode(ErrorCode.ARCHIVE_ERROR)) {
   *   showMessage('压缩文件处理失败，请确保文件格式正确且未损坏');
   * }
   * ```
   */
  ARCHIVE_ERROR = 'ARCHIVE_ERROR',

  // 验证错误
  /**
   * 验证错误
   *
   * 输入验证失败、格式不正确、约束不满足等
   *
   * @example
   * ```typescript
   * if (error.isErrorCode(ErrorCode.VALIDATION_ERROR)) {
   *   showMessage('输入验证失败，请检查输入格式');
   * }
   * ```
   */
  VALIDATION_ERROR = 'VALIDATION_ERROR',

  // 安全错误
  /**
   * 安全错误
   *
   * 路径遍历、权限不足、非法操作等
   *
   * @example
   * ```typescript
   * if (error.isErrorCode(ErrorCode.SECURITY_ERROR)) {
   *   showMessage('安全检查失败，操作被拒绝');
   * }
   * ```
   */
  SECURITY_ERROR = 'SECURITY_ERROR',

  // 未找到
  /**
   * 未找到错误
   *
   * 工作区、文件、任务等资源不存在
   *
   * @example
   * ```typescript
   * if (error.isErrorCode(ErrorCode.NOT_FOUND)) {
   *   showMessage('未找到指定资源');
   * }
   * ```
   */
  NOT_FOUND = 'NOT_FOUND',

  // 路径无效
  /**
   * 路径无效错误
   *
   * 路径格式错误、路径不存在、路径过长等
   *
   * @example
   * ```typescript
   * if (error.isErrorCode(ErrorCode.INVALID_PATH)) {
   *   showMessage('路径无效或不存在，请检查路径是否正确');
   * }
   * ```
   */
  INVALID_PATH = 'INVALID_PATH',

  // 编码错误
  /**
   * 编码错误
   *
   * 文件编码检测失败、字符编码转换错误等
   *
   * @example
   * ```typescript
   * if (error.isErrorCode(ErrorCode.ENCODING_ERROR)) {
   *   showMessage('文件编码读取失败，请确保文件使用支持的编码格式');
   * }
   * ```
   */
  ENCODING_ERROR = 'ENCODING_ERROR',

  // 查询执行错误
  /**
   * 查询执行错误
   *
   * 查询语法错误、查询超时、查询资源不足等
   *
   * @example
   * ```typescript
   * if (error.isErrorCode(ErrorCode.QUERY_EXECUTION_ERROR)) {
   *   showMessage('查询执行失败，请检查查询语法或稍后重试');
   * }
   * ```
   */
  QUERY_EXECUTION_ERROR = 'QUERY_EXECUTION_ERROR',

  // 文件监听错误
  /**
   * 文件监听错误
   *
   * 文件监听启动失败、监听器崩溃等
   *
   * @example
   * ```typescript
   * if (error.isErrorCode(ErrorCode.FILE_WATCHER_ERROR)) {
   *   showMessage('文件监听启动失败，请检查文件权限');
   * }
   * ```
   */
  FILE_WATCHER_ERROR = 'FILE_WATCHER_ERROR',

  // 索引错误
  /**
   * 索引错误
   *
   * 索引损坏、索引不可用、索引创建失败等
   *
   * @example
   * ```typescript
   * if (error.isErrorCode(ErrorCode.INDEX_ERROR)) {
   *   showMessage('索引错误，请尝试重新加载工作区');
   * }
   * ```
   */
  INDEX_ERROR = 'INDEX_ERROR',

  // 模式错误
  /**
   * 模式错误
   *
   * 正则表达式语法错误、模式编译失败等
   *
   * @example
   * ```typescript
   * if (error.isErrorCode(ErrorCode.PATTERN_ERROR)) {
   *   showMessage('正则表达式格式错误，请检查语法');
   * }
   * ```
   */
  PATTERN_ERROR = 'PATTERN_ERROR',

  // 数据库错误
  /**
   * 数据库错误
   *
   * 数据库连接失败、查询失败、事务失败等
   *
   * @example
   * ```typescript
   * if (error.isErrorCode(ErrorCode.DATABASE_ERROR)) {
   *   showMessage('数据库操作失败，请检查数据库状态');
   * }
   * ```
   */
  DATABASE_ERROR = 'DATABASE_ERROR',

  // 配置错误
  /**
   * 配置错误
   *
   * 配置加载失败、配置保存失败、配置格式错误等
   *
   * @example
   * ```typescript
   * if (error.isErrorCode(ErrorCode.CONFIG_ERROR)) {
   *   showMessage('配置保存失败，请检查配置格式');
   * }
   * ```
   */
  CONFIG_ERROR = 'CONFIG_ERROR',

  // 网络错误
  /**
   * 网络错误
   *
   * 网络连接失败、请求超时等（虽然应用是离线的，但可能涉及本地网络）
   *
   * @example
   * ```typescript
   * if (error.isErrorCode(ErrorCode.NETWORK_ERROR)) {
   *   showMessage('网络操作失败，请检查网络连接');
   * }
   * ```
   */
  NETWORK_ERROR = 'NETWORK_ERROR',

  // 内部错误
  /**
   * 内部错误
   *
   * 未预期的内部错误、bug、panic 捕获等
   *
   * @example
   * ```typescript
   * if (error.isErrorCode(ErrorCode.INTERNAL_ERROR)) {
   *   showMessage('系统内部错误，请联系支持团队');
   * }
   * ```
   */
  INTERNAL_ERROR = 'INTERNAL_ERROR',

  // 资源清理错误
  /**
   * 资源清理错误
   *
   * 临时文件清理失败、内存释放失败等
   *
   * @example
   * ```typescript
   * if (error.isErrorCode(ErrorCode.RESOURCE_CLEANUP_ERROR)) {
   *   showMessage('资源清理失败，请手动清理临时文件');
   * }
   * ```
   */
  RESOURCE_CLEANUP_ERROR = 'RESOURCE_CLEANUP_ERROR',

  // 并发错误
  /**
   * 并发错误
   *
   * 锁竞争、死锁、资源冲突等
   *
   * @example
   * ```typescript
   * if (error.isErrorCode(ErrorCode.CONCURRENCY_ERROR)) {
   *   showMessage('操作冲突，请稍后重试');
   * }
   * ```
   */
  CONCURRENCY_ERROR = 'CONCURRENCY_ERROR',

  // 解析错误
  /**
   * 解析错误
   *
   * 数据格式错误、JSON 解析失败、序列化失败等
   *
   * @example
   * ```typescript
   * if (error.isErrorCode(ErrorCode.PARSE_ERROR)) {
   *   showMessage('数据解析失败，请检查数据格式');
   * }
   * ```
   */
  PARSE_ERROR = 'PARSE_ERROR',

  // 超时错误
  /**
   * 超时错误
   *
   * 操作超时、响应时间过长等
   *
   * @example
   * ```typescript
   * if (error.isErrorCode(ErrorCode.TIMEOUT_ERROR)) {
   *   showMessage('操作超时，请稍后重试');
   * }
   * ```
   */
  TIMEOUT_ERROR = 'TIMEOUT_ERROR',

  // 未知错误
  /**
   * 未知错误
   *
   * 无法识别的错误类型
   *
   * @example
   * ```typescript
   * if (error.isErrorCode(ErrorCode.UNKNOWN)) {
   *   showMessage('未知错误，请查看日志获取更多信息');
   * }
   * ```
   */
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
// 错误码到默认消息的映射
// ============================================================================

const ERROR_CODE_MESSAGES: Record<string, string> = {
  [ErrorCode.IO_ERROR]: '文件操作失败，请检查文件权限和磁盘空间',
  [ErrorCode.SEARCH_ERROR]: '搜索失败，请尝试简化搜索词或检查工作区状态',
  [ErrorCode.ARCHIVE_ERROR]: '压缩文件处理失败，请确保文件格式正确',
  [ErrorCode.VALIDATION_ERROR]: '输入验证失败，请检查输入格式',
  [ErrorCode.SECURITY_ERROR]: '安全检查失败，操作被拒绝',
  [ErrorCode.NOT_FOUND]: '未找到指定资源',
  [ErrorCode.INVALID_PATH]: '路径无效或不存在',
  [ErrorCode.ENCODING_ERROR]: '文件编码读取失败',
  [ErrorCode.QUERY_EXECUTION_ERROR]: '查询执行失败，请检查查询语法',
  [ErrorCode.FILE_WATCHER_ERROR]: '文件监听启动失败',
  [ErrorCode.INDEX_ERROR]: '索引错误，请尝试重新加载工作区',
  [ErrorCode.PATTERN_ERROR]: '正则表达式格式错误',
  [ErrorCode.DATABASE_ERROR]: '数据库操作失败',
  [ErrorCode.CONFIG_ERROR]: '配置保存失败',
  [ErrorCode.NETWORK_ERROR]: '网络操作失败',
  [ErrorCode.INTERNAL_ERROR]: '系统内部错误',
  [ErrorCode.RESOURCE_CLEANUP_ERROR]: '资源清理失败',
  [ErrorCode.CONCURRENCY_ERROR]: '并发操作冲突',
  [ErrorCode.PARSE_ERROR]: '数据解析失败',
  [ErrorCode.TIMEOUT_ERROR]: '操作超时',
  [ErrorCode.UNKNOWN]: '未知错误',
};

// ============================================================================
// API 错误类
// ============================================================================

/**
 * API 错误类
 *
 * 封装所有 Tauri 命令调用错误，提供结构化访问
 */
export class ApiError extends Error implements StructuredError {
  /** 错误码 */
  code: string;

  /** 帮助提示 */
  help?: string;

  /** 错误详情 */
  details?: unknown;

  /** 原始错误 */
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

    // 尝试从错误消息中解析结构化信息
    this.parseStructuredError(cause);
  }

  /**
   * 解析结构化错误
   */
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

  /**
   * 获取错误分类
   */
  getCategory(): ErrorCategory {
    return ERROR_CODE_CATEGORIES[this.code] || ErrorCategory.UNKNOWN;
  }

  /**
   * 判断是否为特定错误码
   */
  isErrorCode(code: ErrorCode): boolean {
    return this.code === code;
  }

  /**
   * 判断是否为用户错误
   */
  isUserError(): boolean {
    return this.getCategory() === ErrorCategory.USER;
  }

  /**
   * 判断是否为系统错误
   */
  isSystemError(): boolean {
    return this.getCategory() === ErrorCategory.SYSTEM;
  }

  /**
   * 判断是否为网络错误
   */
  isNetworkError(): boolean {
    return this.getCategory() === ErrorCategory.NETWORK;
  }

  /**
   * 判断是否为文件系统错误
   */
  isFilesystemError(): boolean {
    return this.getCategory() === ErrorCategory.FILESYSTEM;
  }

  /**
   * 获取用户友好的错误消息
   */
  getUserMessage(): string {
    // 如果有原始消息，使用原始消息
    if (this.message && this.message !== this.code) {
      return this.message;
    }

    // 否则使用错误码的默认消息
    return ERROR_CODE_MESSAGES[this.code] || this.message;
  }

  /**
   * 获取完整的错误提示（包含帮助信息）
   */
  getFullMessage(): string {
    const userMessage = this.getUserMessage();
    if (this.help) {
      return `${userMessage}\n提示：${this.help}`;
    }
    return userMessage;
  }

  /**
   * 转换为可序列化的对象
   */
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

/**
 * 从未知错误创建 ApiError
 *
 * @param command - 命令名称
 * @param error - 未知错误
 * @returns ApiError 实例
 */
export function createApiError(command: string, error: unknown): ApiError {
  if (error instanceof ApiError) {
    return error;
  }

  const message = error instanceof Error ? error.message : String(error);
  return new ApiError(command, message, error);
}

/**
 * 判断错误是否可重试
 *
 * @param error - 错误对象
 * @returns 是否可重试
 */
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

/**
 * 获取错误消息用于显示
 *
 * @param error - 错误对象
 * @returns 用户友好的错误消息
 */
export function getErrorMessage(error: unknown): string {
  if (error instanceof ApiError) {
    return error.getUserMessage();
  }

  if (error instanceof Error) {
    return error.message;
  }

  return String(error);
}

/**
 * 获取完整错误提示（包含帮助信息）
 *
 * @param error - 错误对象
 * @returns 完整的错误提示
 */
export function getFullErrorMessage(error: unknown): string {
  if (error instanceof ApiError) {
    return error.getFullMessage();
  }

  return getErrorMessage(error);
}

/**
 * 错误处理装饰器
 *
 * 自动捕获错误并转换为 ApiError
 *
 * @param command - 命令名称
 * @param fn - 要执行的异步函数
 * @returns 包装后的函数
 */
export function withErrorHandler<T extends (...args: any[]) => Promise<any>>(
  command: string,
  fn: T
): T {
  return (async (...args: any[]) => {
    try {
      return await fn(...args);
    } catch (error) {
      throw createApiError(command, error);
    }
  }) as T;
}

// ============================================================================
// 导出
// ============================================================================

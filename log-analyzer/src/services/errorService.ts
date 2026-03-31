/**
 * 错误处理服务
 *
 * 将后端错误映射到i18n本地化的错误消息
 * 支持错误码到i18n键的映射和动态参数替换
 */

import i18n from '../i18n';

/**
 * 后端错误结构 (来自 Rust CommandError)
 */
export interface BackendError {
  code: string;
  message: string;
  help?: string;
  details?: Record<string, unknown>;
}

/**
 * 错误码到i18n键的映射
 * 键格式: errors.<module>.<error_key>
 */
const ERROR_CODE_MAP: Record<string, string> = {
  // 验证错误
  VALIDATION_ERROR: 'errors.search.query_empty',
  NOT_FOUND: 'errors.workspace.not_found',
  DATABASE_ERROR: 'errors.search.database_error',
  INTERNAL_ERROR: 'errors.unknown',
  TIMEOUT_ERROR: 'errors.search.timeout',
  IO_ERROR: 'errors.io.error',

  // 搜索相关
  SEARCH_ERROR: 'errors.search.execution_error',

  // 工作区相关
  WORKSPACE_NOT_FOUND: 'errors.workspace.not_found',
  WORKSPACE_NOT_CAS: 'errors.workspace.not_cas_format',

  // 任务相关
  TASK_MANAGER_NOT_INITIALIZED: 'errors.task.manager_not_initialized',

  // 导入相关
  IMPORT_PATH_NOT_EXIST: 'errors.import.path_not_exist',

  // 配置相关
  CONFIG_ERROR: 'errors.config.validation_failed',
};

/**
 * 错误消息模式到i18n键的映射
 * 用于匹配动态生成的错误消息
 */
const ERROR_PATTERN_MAP: Array<{ pattern: RegExp; key: string; params?: (match: RegExpMatchArray) => Record<string, string> }> = [
  // 搜索相关
  { pattern: /Search query cannot be empty/i, key: 'errors.search.query_empty' },
  { pattern: /Search query too long.*max 1000/i, key: 'errors.search.query_too_long' },
  { pattern: /Search query is empty after processing/i, key: 'errors.search.query_empty_after_processing' },
  { pattern: /Workspace not found:\s*(.+)/i, key: 'errors.search.workspace_not_found', params: (m) => ({ workspaceId: m[1] }) },
  { pattern: /No workspaces available/i, key: 'errors.search.no_workspaces_available' },
  { pattern: /Failed to open metadata store:\s*(.+)/i, key: 'errors.search.metadata_store_failed', params: (m) => ({ error: m[1] }) },
  { pattern: /Internal error occurred while accessing workspace:\s*(.+)/i, key: 'errors.search.database_error', params: (m) => ({ workspaceId: m[1] }) },
  { pattern: /Query execution error:\s*(.+)/i, key: 'errors.search.execution_error', params: (m) => ({ error: m[1] }) },
  { pattern: /Search task panicked:\s*(.+)/i, key: 'errors.search.task_panicked', params: (m) => ({ error: m[1] }) },
  { pattern: /Search timed out after (\d+) seconds/i, key: 'errors.search.timeout', params: (m) => ({ seconds: m[1] }) },
  { pattern: /Search with ID (.+) not found or already completed/i, key: 'errors.search.session_not_found', params: (m) => ({ searchId: m[1] }) },
  { pattern: /Search session '(.+)' not found or expired/i, key: 'errors.search.session_expired', params: (m) => ({ searchId: m[1] }) },
  { pattern: /Failed to read search page:\s*(.+)/i, key: 'errors.search.page_read_failed', params: (m) => ({ error: m[1] }) },
  { pattern: /Search not found in cache/i, key: 'errors.search.cache_not_found' },

  // 导入相关
  { pattern: /Path does not exist:\s*(.+)/i, key: 'errors.import.path_not_exist', params: (m) => ({ path: m[1] }) },
  { pattern: /Path canonicalization failed:\s*(.+)/i, key: 'errors.import.path_canonicalization_failed', params: (m) => ({ error: m[1] }) },
  { pattern: /Failed to get app data dir:\s*(.+)/i, key: 'errors.import.app_data_dir_failed', params: (m) => ({ error: m[1] }) },
  { pattern: /Failed to create workspace dir:\s*(.+)/i, key: 'errors.import.workspace_dir_failed', params: (m) => ({ error: m[1] }) },
  { pattern: /Failed to create task:\s*(.+)/i, key: 'errors.import.task_creation_failed', params: (m) => ({ error: m[1] }) },
  { pattern: /Task manager not initialized/i, key: 'errors.import.task_manager_not_initialized' },
  { pattern: /Failed to update task progress:\s*(.+)/i, key: 'errors.import.task_update_failed', params: (m) => ({ error: m[1] }) },
  { pattern: /Failed to create metadata store:\s*(.+)/i, key: 'errors.import.metadata_store_failed', params: (m) => ({ error: m[1] }) },
  { pattern: /Failed to cleanup workspace directory:\s*(.+)/i, key: 'errors.import.cleanup_failed', params: (m) => ({ error: m[1] }) },
  { pattern: /Failed to process path:\s*(.+)/i, key: 'errors.import.process_failed', params: (m) => ({ error: m[1] }) },
  { pattern: /Failed to verify integrity after import:\s*(.+)/i, key: 'errors.import.verification_failed', params: (m) => ({ error: m[1] }) },

  // 工作区相关
  { pattern: /Workspace ID cannot be empty/i, key: 'errors.workspace.id_empty' },
  { pattern: /Workspace ID too long.*max 50/i, key: 'errors.workspace.id_too_long' },
  { pattern: /Workspace ID can only contain alphanumeric/i, key: 'errors.workspace.id_invalid' },
  { pattern: /Workspace (.+) is not in CAS format/i, key: 'errors.workspace.not_cas_format', params: (m) => ({ workspaceId: m[1] }) },
  { pattern: /Workspace store not initialized:\s*(.+)/i, key: 'errors.workspace.store_not_initialized', params: (m) => ({ workspaceId: m[1] }) },
  { pattern: /Failed to count files:\s*(.+)/i, key: 'errors.workspace.file_count_failed', params: (m) => ({ error: m[1] }) },
  { pattern: /Failed to get time range from index:\s*(.+)/i, key: 'errors.workspace.time_range_failed', params: (m) => ({ error: m[1] }) },

  // 验证相关
  { pattern: /Path traversal pattern detected:\s*(.+)/i, key: 'errors.validation.path_traversal', params: (m) => ({ pattern: m[1] }) },
  { pattern: /Null byte injection detected/i, key: 'errors.validation.null_byte' },
  { pattern: /Control characters detected in path/i, key: 'errors.validation.control_chars' },
  { pattern: /Failed to canonicalize.*:\s*(.+)/i, key: 'errors.validation.path_canonicalization_failed', params: (m) => ({ error: m[1] }) },
  { pattern: /Filename cannot be empty/i, key: 'errors.validation.filename_empty' },
  { pattern: /Filename contains only invalid characters/i, key: 'errors.validation.filename_invalid' },
  { pattern: /Filename too long.*max 255/i, key: 'errors.validation.filename_too_long' },
  { pattern: /Reserved filename:\s*(.+)/i, key: 'errors.validation.filename_reserved', params: (m) => ({ name: m[1] }) },
  { pattern: /Too many duplicate filenames at index (\d+)/i, key: 'errors.validation.too_many_duplicates', params: (m) => ({ index: m[1] }) },

  // 导出相关
  { pattern: /Export path contains illegal path traversal/i, key: 'errors.export.path_traversal' },
  { pattern: /Export directory does not exist:\s*(.+)/i, key: 'errors.export.directory_not_exist', params: (m) => ({ path: m[1] }) },
  { pattern: /Unsupported export format:\s*(.+)/i, key: 'errors.export.unsupported_format', params: (m) => ({ format: m[1] }) },
  { pattern: /Export task panicked:\s*(.+)/i, key: 'errors.export.task_panicked', params: (m) => ({ error: m[1] }) },
  { pattern: /Failed to create CSV file:\s*(.+)/i, key: 'errors.export.csv_create_failed', params: (m) => ({ error: m[1] }) },
  { pattern: /Failed to write CSV.*:\s*(.+)/i, key: 'errors.export.csv_write_failed', params: (m) => ({ error: m[1] }) },
  { pattern: /Failed to serialize JSON:\s*(.+)/i, key: 'errors.export.json_serialize_failed', params: (m) => ({ error: m[1] }) },
  { pattern: /Failed to write JSON file:\s*(.+)/i, key: 'errors.export.json_write_failed', params: (m) => ({ error: m[1] }) },

  // 监听相关
  { pattern: /Workspace is already being watched/i, key: 'errors.watch.already_watched' },
  { pattern: /Failed to create file watcher:\s*(.+)/i, key: 'errors.watch.watcher_create_failed', params: (m) => ({ error: m[1] }) },
  { pattern: /Failed to start watching path:\s*(.+)/i, key: 'errors.watch.start_failed', params: (m) => ({ error: m[1] }) },
  { pattern: /No active watcher found for this workspace/i, key: 'errors.watch.no_active_watcher' },

  // 性能相关
  { pattern: /Failed to initialize metrics store:\s*(.+)/i, key: 'errors.performance.metrics_store_init_failed', params: (m) => ({ error: m[1] }) },
  { pattern: /Metrics store not initialized/i, key: 'errors.performance.metrics_store_not_initialized' },
  { pattern: /Database error:\s*(.+)/i, key: 'errors.performance.database_error', params: (m) => ({ error: m[1] }) },

  // 任务相关
  { pattern: /Failed to cancel task:\s*(.+)/i, key: 'errors.task.cancel_failed', params: (m) => ({ error: m[1] }) },
  { pattern: /TaskManager actor has stopped/i, key: 'errors.task.actor_stopped' },
  { pattern: /Operation timed out/i, key: 'errors.task.operation_timeout' },
  { pattern: /Actor dropped response channel/i, key: 'errors.task.actor_dropped' },
  { pattern: /Failed to send shutdown message:\s*(.+)/i, key: 'errors.task.shutdown_failed', params: (m) => ({ error: m[1] }) },
  { pattern: /TaskManager channel is full/i, key: 'errors.task.channel_full' },
];

/**
 * 将后端错误转换为本地化的错误消息
 *
 * @param error - 后端错误对象或错误消息字符串
 * @returns 本地化的错误消息
 */
export function getLocalizedErrorMessage(error: BackendError | string): string {
  if (typeof error === 'string') {
    return localizeErrorMessage(error);
  }

  // 首先尝试使用错误码映射
  if (error.code && ERROR_CODE_MAP[error.code]) {
    const key = ERROR_CODE_MAP[error.code];
    const translated = i18n.t(key);
    // 如果翻译存在且不等于键本身，使用翻译
    if (translated && translated !== key) {
      return translated;
    }
  }

  // 然后尝试使用消息模式匹配
  const localized = localizeErrorMessage(error.message);
  if (localized !== error.message) {
    return localized;
  }

  // 如果都没有匹配，返回原始消息
  return error.message;
}

/**
 * 根据错误消息模式获取本地化的错误消息
 *
 * @param message - 错误消息
 * @returns 本地化的错误消息
 */
function localizeErrorMessage(message: string): string {
  for (const { pattern, key, params } of ERROR_PATTERN_MAP) {
    const match = message.match(pattern);
    if (match) {
      const parameters = params ? params(match) : {};
      const translated = i18n.t(key, parameters);
      if (translated && translated !== key) {
        return translated;
      }
    }
  }
  return message;
}

/**
 * 获取错误帮助信息
 *
 * @param error - 后端错误对象
 * @returns 本地化的帮助信息（如果有）
 */
export function getLocalizedHelpMessage(error: BackendError): string | undefined {
  if (!error.help) {
    return undefined;
  }
  // 帮助信息通常不需要复杂的映射，直接返回
  return error.help;
}

/**
 * 创建用户友好的错误对象
 *
 * @param error - 后端错误
 * @returns 格式化后的错误对象
 */
export function createUserFriendlyError(error: BackendError | string): {
  title: string;
  message: string;
  help?: string;
  code?: string;
} {
  const title = i18n.t('errors.title');

  if (typeof error === 'string') {
    return {
      title,
      message: localizeErrorMessage(error),
    };
  }

  return {
    title,
    message: getLocalizedErrorMessage(error),
    help: error.help ? getLocalizedHelpMessage(error) : undefined,
    code: error.code,
  };
}

/**
 * 检查错误是否是可重试的
 *
 * @param error - 后端错误
 * @returns 是否可重试
 */
export function isRetryableError(error: BackendError | string): boolean {
  if (typeof error === 'string') {
    return false;
  }

  const retryableCodes = ['TIMEOUT_ERROR', 'IO_ERROR', 'DATABASE_ERROR'];
  return retryableCodes.includes(error.code);
}

/**
 * 错误处理钩子 - 用于React组件
 *
 * @returns 错误处理函数
 */
export function useErrorHandler() {
  return {
    getMessage: getLocalizedErrorMessage,
    getHelp: getLocalizedHelpMessage,
    createError: createUserFriendlyError,
    isRetryable: isRetryableError,
  };
}

export default {
  getLocalizedErrorMessage,
  getLocalizedHelpMessage,
  createUserFriendlyError,
  isRetryableError,
  useErrorHandler,
};

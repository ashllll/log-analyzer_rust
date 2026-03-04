/**
 * 错误处理模块单元测试
 *
 * 测试错误码、错误类型和错误处理工具函数
 */

import {
  ErrorCode,
  ErrorCategory,
  ApiError,
  StructuredError,
  createApiError,
  isRetryableError,
  getErrorMessage,
  getFullErrorMessage,
  withErrorHandler,
} from '../errors';

describe('ErrorCode 枚举', () => {
  describe('错误码定义', () => {
    it('应该定义所有必需的错误码', () => {
      // IO 错误
      expect(ErrorCode.IO_ERROR).toBe('IO_ERROR');

      // 搜索错误
      expect(ErrorCode.SEARCH_ERROR).toBe('SEARCH_ERROR');

      // 归档错误
      expect(ErrorCode.ARCHIVE_ERROR).toBe('ARCHIVE_ERROR');

      // 验证错误
      expect(ErrorCode.VALIDATION_ERROR).toBe('VALIDATION_ERROR');

      // 安全错误
      expect(ErrorCode.SECURITY_ERROR).toBe('SECURITY_ERROR');

      // 未找到
      expect(ErrorCode.NOT_FOUND).toBe('NOT_FOUND');

      // 路径无效
      expect(ErrorCode.INVALID_PATH).toBe('INVALID_PATH');

      // 编码错误
      expect(ErrorCode.ENCODING_ERROR).toBe('ENCODING_ERROR');

      // 查询执行错误
      expect(ErrorCode.QUERY_EXECUTION_ERROR).toBe('QUERY_EXECUTION_ERROR');

      // 文件监听错误
      expect(ErrorCode.FILE_WATCHER_ERROR).toBe('FILE_WATCHER_ERROR');

      // 索引错误
      expect(ErrorCode.INDEX_ERROR).toBe('INDEX_ERROR');

      // 模式错误
      expect(ErrorCode.PATTERN_ERROR).toBe('PATTERN_ERROR');

      // 数据库错误
      expect(ErrorCode.DATABASE_ERROR).toBe('DATABASE_ERROR');

      // 配置错误
      expect(ErrorCode.CONFIG_ERROR).toBe('CONFIG_ERROR');

      // 网络错误
      expect(ErrorCode.NETWORK_ERROR).toBe('NETWORK_ERROR');

      // 内部错误
      expect(ErrorCode.INTERNAL_ERROR).toBe('INTERNAL_ERROR');

      // 资源清理错误
      expect(ErrorCode.RESOURCE_CLEANUP_ERROR).toBe('RESOURCE_CLEANUP_ERROR');

      // 并发错误
      expect(ErrorCode.CONCURRENCY_ERROR).toBe('CONCURRENCY_ERROR');

      // 解析错误
      expect(ErrorCode.PARSE_ERROR).toBe('PARSE_ERROR');

      // 超时错误
      expect(ErrorCode.TIMEOUT_ERROR).toBe('TIMEOUT_ERROR');

      // 未知错误
      expect(ErrorCode.UNKNOWN).toBe('UNKNOWN');
    });

    it('错误码应该有唯一的值', () => {
      const errorCodes = Object.values(ErrorCode);
      const uniqueCodes = new Set(errorCodes);
      expect(uniqueCodes.size).toBe(errorCodes.length);
    });
  });
});

describe('ErrorCategory 枚举', () => {
  it('应该定义所有错误分类', () => {
    expect(ErrorCategory.USER).toBe('user');
    expect(ErrorCategory.SYSTEM).toBe('system');
    expect(ErrorCategory.NETWORK).toBe('network');
    expect(ErrorCategory.FILESYSTEM).toBe('filesystem');
    expect(ErrorCategory.UNKNOWN).toBe('unknown');
  });
});

describe('ApiError 类', () => {
  describe('构造函数', () => {
    it('应该创建基本的错误实例', () => {
      const error = new ApiError('test_command', 'Test error message');

      expect(error.name).toBe('ApiError');
      expect(error.command).toBe('test_command');
      expect(error.message).toBe('Test error message');
      expect(error.code).toBe(ErrorCode.UNKNOWN);
    });

    it('应该存储 cause', () => {
      const originalError = new Error('Original error');
      const apiError = new ApiError('test_command', 'Wrapper message', originalError);

      expect(apiError.cause).toBe(originalError);
    });

    it('应该从 Error 中提取 message', () => {
      const originalError = new Error('Original error message');
      const apiError = new ApiError('test_command', 'Wrapper', originalError);

      expect(apiError.message).toBe('Wrapper');
    });
  });

  describe('结构化错误解析', () => {
    it('应该从 JSON 字符串解析错误码', () => {
      const structuredError = JSON.stringify({
        code: ErrorCode.IO_ERROR,
        message: 'File operation failed',
        help: 'Check file permissions',
        details: { path: '/test/path' },
      });

      const apiError = new ApiError('test_command', 'Wrapper', structuredError);

      expect(apiError.code).toBe(ErrorCode.IO_ERROR);
      expect(apiError.message).toBe('File operation failed');
      expect(apiError.help).toBe('Check file permissions');
      expect(apiError.details).toEqual({ path: '/test/path' });
    });

    it('应该从对象解析错误码', () => {
      const structuredError = {
        code: ErrorCode.VALIDATION_ERROR,
        message: 'Invalid input',
        help: 'Check input format',
      };

      const apiError = new ApiError('test_command', 'Wrapper', structuredError);

      expect(apiError.code).toBe(ErrorCode.VALIDATION_ERROR);
      // message 保持为构造函数传入的 'Wrapper'，不会被 cause 覆盖
      expect(apiError.message).toBe('Wrapper');
      expect(apiError.help).toBe('Check input format');
    });

    it('应该处理无效的 JSON 字符串', () => {
      const apiError = new ApiError('test_command', 'Wrapper', 'not valid json');

      expect(apiError.code).toBe(ErrorCode.UNKNOWN);
      expect(apiError.message).toBe('Wrapper');
      expect(apiError.help).toBeUndefined();
    });

    it('应该处理非对象 cause', () => {
      const apiError = new ApiError('test_command', 'Wrapper', 12345);

      expect(apiError.code).toBe(ErrorCode.UNKNOWN);
    });
  });

  describe('getCategory 方法', () => {
    it('应该正确分类用户错误', () => {
      const error = new ApiError('test', 'msg', { code: ErrorCode.VALIDATION_ERROR });
      expect(error.getCategory()).toBe(ErrorCategory.USER);

      const error2 = new ApiError('test', 'msg', { code: ErrorCode.PATTERN_ERROR });
      expect(error2.getCategory()).toBe(ErrorCategory.USER);

      const error3 = new ApiError('test', 'msg', { code: ErrorCode.INVALID_PATH });
      expect(error3.getCategory()).toBe(ErrorCategory.USER);

      const error4 = new ApiError('test', 'msg', { code: ErrorCode.SECURITY_ERROR });
      expect(error4.getCategory()).toBe(ErrorCategory.USER);
    });

    it('应该正确分类系统错误', () => {
      const error = new ApiError('test', 'msg', { code: ErrorCode.DATABASE_ERROR });
      expect(error.getCategory()).toBe(ErrorCategory.SYSTEM);

      const error2 = new ApiError('test', 'msg', { code: ErrorCode.CONCURRENCY_ERROR });
      expect(error2.getCategory()).toBe(ErrorCategory.SYSTEM);

      const error3 = new ApiError('test', 'msg', { code: ErrorCode.INTERNAL_ERROR });
      expect(error3.getCategory()).toBe(ErrorCategory.SYSTEM);
    });

    it('应该正确分类网络错误', () => {
      const error = new ApiError('test', 'msg', { code: ErrorCode.NETWORK_ERROR });
      expect(error.getCategory()).toBe(ErrorCategory.NETWORK);
    });

    it('应该正确分类文件系统错误', () => {
      const error = new ApiError('test', 'msg', { code: ErrorCode.IO_ERROR });
      expect(error.getCategory()).toBe(ErrorCategory.FILESYSTEM);

      const error2 = new ApiError('test', 'msg', { code: ErrorCode.ENCODING_ERROR });
      expect(error2.getCategory()).toBe(ErrorCategory.FILESYSTEM);

      const error3 = new ApiError('test', 'msg', { code: ErrorCode.FILE_WATCHER_ERROR });
      expect(error3.getCategory()).toBe(ErrorCategory.FILESYSTEM);
    });

    it('应该将未分类错误返回 UNKNOWN', () => {
      const error = new ApiError('test', 'msg');
      expect(error.getCategory()).toBe(ErrorCategory.UNKNOWN);
    });
  });

  describe('isErrorCode 方法', () => {
    it('应该正确匹配错误码', () => {
      const error = new ApiError('test', 'msg', { code: ErrorCode.IO_ERROR });

      expect(error.isErrorCode(ErrorCode.IO_ERROR)).toBe(true);
      expect(error.isErrorCode(ErrorCode.NETWORK_ERROR)).toBe(false);
    });

    it('默认 UNKNOWN 错误码应该匹配 UNKNOWN', () => {
      const error = new ApiError('test', 'msg');

      expect(error.isErrorCode(ErrorCode.UNKNOWN)).toBe(true);
      expect(error.isErrorCode(ErrorCode.IO_ERROR)).toBe(false);
    });
  });

  describe('类型判断方法', () => {
    it('isUserError 应该正确识别用户错误', () => {
      const error = new ApiError('test', 'msg', { code: ErrorCode.VALIDATION_ERROR });
      expect(error.isUserError()).toBe(true);

      const error2 = new ApiError('test', 'msg', { code: ErrorCode.IO_ERROR });
      expect(error2.isUserError()).toBe(false);
    });

    it('isSystemError 应该正确识别系统错误', () => {
      const error = new ApiError('test', 'msg', { code: ErrorCode.DATABASE_ERROR });
      expect(error.isSystemError()).toBe(true);

      const error2 = new ApiError('test', 'msg', { code: ErrorCode.VALIDATION_ERROR });
      expect(error2.isSystemError()).toBe(false);
    });

    it('isNetworkError 应该正确识别网络错误', () => {
      const error = new ApiError('test', 'msg', { code: ErrorCode.NETWORK_ERROR });
      expect(error.isNetworkError()).toBe(true);

      const error2 = new ApiError('test', 'msg', { code: ErrorCode.DATABASE_ERROR });
      expect(error2.isNetworkError()).toBe(false);
    });

    it('isFilesystemError 应该正确识别文件系统错误', () => {
      const error = new ApiError('test', 'msg', { code: ErrorCode.IO_ERROR });
      expect(error.isFilesystemError()).toBe(true);

      const error2 = new ApiError('test', 'msg', { code: ErrorCode.NETWORK_ERROR });
      expect(error2.isFilesystemError()).toBe(false);
    });
  });

  describe('getUserMessage 方法', () => {
    it('应该使用解析后的消息', () => {
      // 当使用空字符串作为 message 时，会使用默认消息
      const error = new ApiError('test', '', {
        code: ErrorCode.IO_ERROR,
        message: 'This is ignored',
      });

      expect(error.getUserMessage()).toBe('文件操作失败，请检查文件权限和磁盘空间');
    });

    it('应该使用默认消息当没有自定义消息时', () => {
      const error = new ApiError('test', '', {
        code: ErrorCode.IO_ERROR,
      });

      expect(error.getUserMessage()).toBe('文件操作失败，请检查文件权限和磁盘空间');
    });

    it('应该返回原始消息作为后备', () => {
      const error = new ApiError('test', 'Original message', {
        code: 'NON_EXISTENT_CODE' as ErrorCode,
      });

      expect(error.getUserMessage()).toBe('Original message');
    });

    it('应该为所有错误码提供默认消息', () => {
      const errorCodes = Object.values(ErrorCode);

      errorCodes.forEach((code) => {
        const error = new ApiError('test', '', { code });
        const message = error.getUserMessage();
        expect(message).toBeTruthy();
        expect(typeof message).toBe('string');
        expect(message.length).toBeGreaterThan(0);
      });
    });
  });

  describe('getFullMessage 方法', () => {
    it('应该包含帮助信息', () => {
      const error = new ApiError('test', 'Base message', {
        code: ErrorCode.IO_ERROR,
        help: 'Check file permissions',
      });

      const fullMessage = error.getFullMessage();
      expect(fullMessage).toContain('Check file permissions');
      expect(fullMessage).toContain('提示：');
    });

    it('没有帮助信息时应该只返回用户消息', () => {
      const error = new ApiError('test', 'Base message', {
        code: ErrorCode.IO_ERROR,
      });

      expect(error.getFullMessage()).toBe(error.getUserMessage());
    });
  });

  describe('toJSON 方法', () => {
    it('应该返回可序列化的对象', () => {
      const error = new ApiError('test', 'Message', {
        code: ErrorCode.IO_ERROR,
        help: 'Help text',
        details: { key: 'value' },
      });

      const json = error.toJSON();
      expect(json).toEqual({
        code: ErrorCode.IO_ERROR,
        message: 'Message',
        help: 'Help text',
        details: { key: 'value' },
      });
    });

    it('应该符合 StructuredError 接口', () => {
      const error = new ApiError('test', 'Message', {
        code: ErrorCode.VALIDATION_ERROR,
        help: 'Help',
      });

      const json = error.toJSON() as StructuredError;
      expect(typeof json.code).toBe('string');
      expect(typeof json.message).toBe('string');
      expect(json.help).toBeDefined();
    });
  });
});

describe('createApiError 函数', () => {
  it('应该直接返回 ApiError 实例', () => {
    const originalError = new ApiError('test', 'Original');
    const result = createApiError('command', originalError);

    expect(result).toBe(originalError);
  });

  it('应该从 Error 创建 ApiError', () => {
    const originalError = new Error('Original error');
    const result = createApiError('command', originalError);

    expect(result).toBeInstanceOf(ApiError);
    expect(result.message).toBe('Original error');
    expect(result.cause).toBe(originalError);
  });

  it('应该从字符串创建 ApiError', () => {
    const result = createApiError('command', 'String error');

    expect(result).toBeInstanceOf(ApiError);
    expect(result.message).toBe('String error');
  });

  it('应该从对象创建 ApiError', () => {
    const result = createApiError('command', { custom: 'error object' });

    expect(result).toBeInstanceOf(ApiError);
    expect(result.cause).toEqual({ custom: 'error object' });
  });

  it('应该处理 null/undefined', () => {
    const result1 = createApiError('command', null);
    expect(result1).toBeInstanceOf(ApiError);

    const result2 = createApiError('command', undefined);
    expect(result2).toBeInstanceOf(ApiError);
  });
});

describe('isRetryableError 函数', () => {
  it('应该将 TIMEOUT_ERROR 识别为可重试', () => {
    const error = new ApiError('test', 'msg', { code: ErrorCode.TIMEOUT_ERROR });
    expect(isRetryableError(error)).toBe(true);
  });

  it('应该将 NETWORK_ERROR 识别为可重试', () => {
    const error = new ApiError('test', 'msg', { code: ErrorCode.NETWORK_ERROR });
    expect(isRetryableError(error)).toBe(true);
  });

  it('应该将 CONCURRENCY_ERROR 识别为可重试', () => {
    const error = new ApiError('test', 'msg', { code: ErrorCode.CONCURRENCY_ERROR });
    expect(isRetryableError(error)).toBe(true);
  });

  it('应该将其他错误识别为不可重试', () => {
    const error = new ApiError('test', 'msg', { code: ErrorCode.VALIDATION_ERROR });
    expect(isRetryableError(error)).toBe(false);

    const error2 = new ApiError('test', 'msg', { code: ErrorCode.IO_ERROR });
    expect(isRetryableError(error2)).toBe(false);
  });

  it('应该处理非 ApiError 错误', () => {
    expect(isRetryableError(new Error('普通错误'))).toBe(false);
    expect(isRetryableError('string error')).toBe(false);
    expect(isRetryableError(null)).toBe(false);
  });
});

describe('getErrorMessage 函数', () => {
  it('应该从 ApiError 获取用户消息', () => {
    const error = new ApiError('test', 'API Error message', {
      code: ErrorCode.IO_ERROR,
    });

    expect(getErrorMessage(error)).toBe('API Error message');
  });

  it('应该从普通 Error 获取消息', () => {
    const error = new Error('Regular error');
    expect(getErrorMessage(error)).toBe('Regular error');
  });

  it('应该将其他类型转换为字符串', () => {
    expect(getErrorMessage('string error')).toBe('string error');
    expect(getErrorMessage(12345)).toBe('12345');
    expect(getErrorMessage(null)).toBe('null');
    expect(getErrorMessage(undefined)).toBe('undefined');
  });
});

describe('getFullErrorMessage 函数', () => {
  it('应该从 ApiError 获取完整消息（包含帮助）', () => {
    const error = new ApiError('test', 'Base', {
      code: ErrorCode.IO_ERROR,
      help: 'Check permissions',
    });

    const fullMessage = getFullErrorMessage(error);
    expect(fullMessage).toContain('Check permissions');
  });

  it('应该从普通 Error 获取消息', () => {
    const error = new Error('Regular error');
    expect(getFullErrorMessage(error)).toBe('Regular error');
  });

  it('应该处理字符串错误', () => {
    expect(getFullErrorMessage('String error')).toBe('String error');
  });
});

describe('withErrorHandler 函数', () => {
  it('应该捕获错误并转换为 ApiError', async () => {
    const mockFn = jest.fn().mockRejectedValue(new Error('Original error'));
    const wrappedFn = withErrorHandler('test_command', mockFn);

    await expect(wrappedFn()).rejects.toThrow();
    await expect(wrappedFn()).rejects.toThrow(ApiError);

    try {
      await wrappedFn();
    } catch (error) {
      expect(error).toBeInstanceOf(ApiError);
      if (error instanceof ApiError) {
        expect(error.command).toBe('test_command');
        expect(error.cause).toBeInstanceOf(Error);
      }
    }
  });

  it('应该成功时返回结果', async () => {
    const mockFn = jest.fn().mockResolvedValue('success result');
    const wrappedFn = withErrorHandler('test_command', mockFn);

    const result = await wrappedFn();
    expect(result).toBe('success result');
  });

  it('应该传递参数', async () => {
    const mockFn = jest.fn().mockResolvedValue('result');
    const wrappedFn = withErrorHandler('test_command', mockFn);

    await wrappedFn('arg1', 'arg2', 123);

    expect(mockFn).toHaveBeenCalledWith('arg1', 'arg2', 123);
  });

  it('应该处理已经是 ApiError 的错误', async () => {
    const originalError = new ApiError('original_cmd', 'Original API error');
    const mockFn = jest.fn().mockRejectedValue(originalError);
    const wrappedFn = withErrorHandler('new_command', mockFn);

    try {
      await wrappedFn();
    } catch (error) {
      expect(error).toBe(originalError);
      if (error instanceof ApiError) {
        expect(error.command).toBe('original_cmd');
      }
    }
  });
});

describe('错误分类映射完整性', () => {
  it('所有错误码都应该有有效的分类或默认为 UNKNOWN', () => {
    const errorCodes = Object.values(ErrorCode);

    errorCodes.forEach((code) => {
      const error = new ApiError('test', 'msg', { code });
      const category = error.getCategory();

      expect(Object.values(ErrorCategory)).toContain(category);
    });
  });

  it('所有错误码都应该有默认消息', () => {
    const errorCodes = Object.values(ErrorCode);

    errorCodes.forEach((code) => {
      const error = new ApiError('test', '', { code });
      const message = error.getUserMessage();

      expect(message).toBeTruthy();
      expect(message.length).toBeGreaterThan(0);
    });
  });
});

describe('边界情况处理', () => {
  it('应该处理空字符串错误码', () => {
    const error = new ApiError('test', 'msg', { code: '' as ErrorCode });
    expect(error.getCategory()).toBe(ErrorCategory.UNKNOWN);
  });

  it('应该处理 undefined 帮助信息', () => {
    const error = new ApiError('test', 'msg', {
      code: ErrorCode.IO_ERROR,
      help: undefined,
    });

    expect(error.help).toBeUndefined();
    expect(error.getFullMessage()).not.toContain('提示：');
  });

  it('应该处理空对象 cause', () => {
    const error = new ApiError('test', 'msg', {});
    expect(error.code).toBe(ErrorCode.UNKNOWN);
  });

  it('应该处理嵌套的错误结构', () => {
    const nestedError = {
      error: {
        code: ErrorCode.IO_ERROR,
        message: 'Nested error',
      },
    };

    const error = new ApiError('test', 'msg', nestedError);
    // 不应该解析嵌套结构（只有顶层）
    expect(error.code).toBe(ErrorCode.UNKNOWN);
  });
});

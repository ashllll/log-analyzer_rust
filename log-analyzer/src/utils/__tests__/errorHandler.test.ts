import { ErrorHandler } from '../errorHandler';

const mockLoggerError = jest.fn();

jest.mock('../logger', () => ({
  logger: {
    error: jest.fn((...args: unknown[]) => mockLoggerError(...args)),
  },
}));

describe('ErrorHandler.handle', () => {
  beforeEach(() => {
    mockLoggerError.mockClear();
  });

  it('应将已知后端错误映射为用户友好的中文消息', () => {
    const cases = [
      {
        input: 'Path canonicalization failed',
        expected: '路径无效或不存在 - 检查路径是否正确',
      },
      {
        input: 'Failed to lock',
        expected: '资源正在使用中 - 稍后重试',
      },
      {
        input: 'unrar command not found',
        expected: 'RAR 支持异常 - RAR 解压组件异常，请重新安装或联系维护',
      },
      {
        input: 'Invalid Regex',
        expected: '搜索表达式语法错误 - 检查正则表达式格式',
      },
      {
        input: 'Disk space',
        expected: '磁盘空间不足 - 清理磁盘空间后重试',
      },
      {
        input: 'Path does not exist',
        expected: '路径不存在 - 选择有效的文件或目录',
      },
      {
        input: 'Workspace ID cannot be empty',
        expected: '工作区 ID 不能为空 - 请选择一个工作区',
      },
      {
        input: 'Search query cannot be empty',
        expected: '搜索查询不能为空 - 输入搜索关键词',
      },
    ];

    for (const { input, expected } of cases) {
      const result = ErrorHandler.handle(input);
      expect(result).toBe(expected);
    }
  });

  it('应对超长未知错误返回固定兜底文案', () => {
    const longError = 'x'.repeat(150);
    const result = ErrorHandler.handle(longError);
    expect(result).toBe('操作失败,请查看控制台详情');
  });

  it('应对短未知错误直接返回原字符串', () => {
    const shortError = 'something wrong';
    const result = ErrorHandler.handle(shortError);
    expect(result).toBe('something wrong');
  });

  it('应调用 logger.error 记录错误', () => {
    ErrorHandler.handle('some error');
    expect(mockLoggerError).toHaveBeenCalledWith(
      'Error occurred:',
      'some error'
    );
  });

  it('应能处理 Error 对象', () => {
    const error = new Error('test error object');
    const result = ErrorHandler.handle(error);
    expect(result).toBe('Error: test error object');
  });

  it('应能处理包含已知模式子串的错误', () => {
    const result = ErrorHandler.handle(
      'Something happened: Path canonicalization failed due to permissions'
    );
    expect(result).toBe('路径无效或不存在 - 检查路径是否正确');
  });
});

describe('ErrorHandler.isRetryable', () => {
  it('应对可重试错误模式返回 true', () => {
    expect(ErrorHandler.isRetryable('Failed to lock the resource')).toBe(true);
    expect(ErrorHandler.isRetryable('Resource busy, try again')).toBe(true);
    expect(ErrorHandler.isRetryable('Connection timeout')).toBe(true);
    expect(ErrorHandler.isRetryable('request timeout after 30s')).toBe(true);
  });

  it('应对不可重试错误返回 false', () => {
    expect(ErrorHandler.isRetryable('Path does not exist')).toBe(false);
    expect(ErrorHandler.isRetryable('Invalid Regex')).toBe(false);
    expect(ErrorHandler.isRetryable('')).toBe(false);
  });
});

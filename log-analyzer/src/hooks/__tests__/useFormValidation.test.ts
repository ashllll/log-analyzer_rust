/**
 * useFormValidation Hook 单元测试
 *
 * 测试表单验证 Hook 的异步验证、状态管理和错误处理
 */

import { renderHook, act, waitFor } from '@testing-library/react';
import { useFormValidation, validateWorkspaceIdFrontend, validatePathFrontend, validateFilenameFrontend } from '../useFormValidation';

// Mock Tauri invoke
jest.mock('@tauri-apps/api/core', () => ({
  invoke: jest.fn(),
}));

// Get the mocked invoke function
const mockInvoke = require('@tauri-apps/api/core').invoke;

// Mock logger
jest.mock('../../utils/logger', () => ({
  logger: {
    debug: jest.fn(),
    error: jest.fn(),
    info: jest.fn(),
    warn: jest.fn(),
  },
}));

describe('useFormValidation Hook', () => {
  beforeEach(() => {
    jest.clearAllMocks();
    mockInvoke.mockResolvedValue(undefined);
  });

  describe('初始状态', () => {
    it('应该初始化为非验证状态', () => {
      const { result } = renderHook(() => useFormValidation());

      expect(result.current.isValidating).toBe(false);
    });

    it('应该提供验证方法', () => {
      const { result } = renderHook(() => useFormValidation());

      expect(typeof result.current.validateWorkspaceId).toBe('function');
      expect(typeof result.current.validatePathSecurity).toBe('function');
      expect(typeof result.current.validateWorkspaceConfig).toBe('function');
    });
  });

  describe('validateWorkspaceId', () => {
    it('应该验证有效的工作区 ID', async () => {
      mockInvoke.mockResolvedValueOnce(true);

      const { result } = renderHook(() => useFormValidation());

      let validationResult;
      await act(async () => {
        validationResult = await result.current.validateWorkspaceId('test-workspace');
      });

      expect(validationResult).toEqual({
        isValid: true,
        errorMessage: undefined,
      });
      expect(result.current.isValidating).toBe(false);
    });

    it('应该拒绝无效的工作区 ID', async () => {
      mockInvoke.mockResolvedValueOnce(false);

      const { result } = renderHook(() => useFormValidation());

      let validationResult;
      await act(async () => {
        validationResult = await result.current.validateWorkspaceId('invalid!@#');
      });

      expect(validationResult).toEqual({
        isValid: false,
        errorMessage: '工作区 ID 格式无效',
      });
    });

    it('应该处理空工作区 ID', async () => {
      const { result } = renderHook(() => useFormValidation());

      let validationResult;
      await act(async () => {
        validationResult = await result.current.validateWorkspaceId('');
      });

      expect(validationResult).toEqual({
        isValid: false,
        errorMessage: '工作区 ID 不能为空',
      });

      // 不应调用后端
      expect(mockInvoke).not.toHaveBeenCalled();
    });

    it('应该设置验证中状态', async () => {
      mockInvoke.mockImplementation(
        () => new Promise((resolve) => {
          setTimeout(() => resolve(true), 100);
        })
      );

      const { result } = renderHook(() => useFormValidation());

      act(() => {
        result.current.validateWorkspaceId('test');
      });

      expect(result.current.isValidating).toBe(true);

      await waitFor(() => {
        expect(result.current.isValidating).toBe(false);
      });
    });

    it('应该处理验证错误', async () => {
      mockInvoke.mockRejectedValueOnce(new Error('Network error'));

      const { result } = renderHook(() => useFormValidation());

      let validationResult;
      await act(async () => {
        validationResult = await result.current.validateWorkspaceId('test');
      });

      expect(validationResult).toEqual({
        isValid: false,
        errorMessage: '验证失败：Error: Network error',
      });
    });
  });

  describe('validatePathSecurity', () => {
    it('应该验证安全路径', async () => {
      mockInvoke.mockResolvedValueOnce({
        isSafe: true,
        issues: [],
      });

      const { result } = renderHook(() => useFormValidation());

      let validationResult;
      await act(async () => {
        validationResult = await result.current.validatePathSecurity('/safe/path');
      });

      expect(validationResult).toEqual({
        isSafe: true,
        issues: [],
      });
    });

    it('应该检测不安全路径', async () => {
      mockInvoke.mockResolvedValueOnce({
        isSafe: false,
        issues: [
          {
            severity: 'error',
            code: 'PATH_TRAVERSAL',
            message: '检测到路径遍历攻击',
          },
        ],
      });

      const { result } = renderHook(() => useFormValidation());

      let validationResult;
      await act(async () => {
        validationResult = await result.current.validatePathSecurity('../../../etc/passwd');
      });

      expect(validationResult.isSafe).toBe(false);
      expect(validationResult.issues).toHaveLength(1);
      expect(validationResult.issues[0].code).toBe('PATH_TRAVERSAL');
    });

    it('应该处理空路径', async () => {
      const { result } = renderHook(() => useFormValidation());

      let validationResult;
      await act(async () => {
        validationResult = await result.current.validatePathSecurity('');
      });

      expect(validationResult).toEqual({
        isSafe: false,
        issues: [
          {
            severity: 'error',
            code: 'EMPTY_PATH',
            message: '路径不能为空',
          },
        ],
      });

      // 不应调用后端
      expect(mockInvoke).not.toHaveBeenCalled();
    });
  });

  describe('validateWorkspaceConfig', () => {
    it('应该验证有效配置', async () => {
      mockInvoke.mockResolvedValueOnce({
        isValid: true,
        issues: [],
      });

      const { result } = renderHook(() => useFormValidation());

      let validationResult;
      await act(async () => {
        validationResult = await result.current.validateWorkspaceConfig('test-workspace');
      });

      expect(validationResult).toEqual({
        isValid: true,
        errorMessage: undefined,
        warningMessage: undefined,
        issues: [],
      });
    });

    it('应该处理错误和警告', async () => {
      mockInvoke.mockResolvedValueOnce({
        isValid: false,
        issues: [
          {
            severity: 'error',
            code: 'INVALID_CONFIG',
            message: '配置无效',
          },
          {
            severity: 'warning',
            code: 'DEPRECATED_SETTING',
            message: '此设置已弃用',
          },
        ],
      });

      const { result } = renderHook(() => useFormValidation());

      let validationResult;
      await act(async () => {
        validationResult = await result.current.validateWorkspaceConfig('test-workspace');
      });

      expect(validationResult.isValid).toBe(false);
      expect(validationResult.errorMessage).toBe('配置无效');
      expect(validationResult.warningMessage).toBe('此设置已弃用');
    });

    it('应该处理验证异常', async () => {
      mockInvoke.mockRejectedValueOnce(new Error('RPC error'));

      const { result } = renderHook(() => useFormValidation());

      let validationResult;
      await act(async () => {
        validationResult = await result.current.validateWorkspaceConfig('test-workspace');
      });

      expect(validationResult.isValid).toBe(false);
      expect(validationResult.errorMessage).toContain('验证失败');
    });
  });
});

describe('前端验证函数', () => {
  describe('validateWorkspaceIdFrontend', () => {
    it('应该接受有效的工作区 ID', () => {
      const result = validateWorkspaceIdFrontend('my-workspace-01');

      expect(result.isValid).toBe(true);
      expect(result.errorMessage).toBeUndefined();
    });

    it('应该拒绝空的工作区 ID', () => {
      const result = validateWorkspaceIdFrontend('');

      expect(result.isValid).toBe(false);
      expect(result.errorMessage).toBe('工作区 ID 不能为空');
    });

    it('应该拒绝过短的工作区 ID', () => {
      const result = validateWorkspaceIdFrontend('ab');

      expect(result.isValid).toBe(false);
      expect(result.errorMessage).toContain('至少需要 3 个字符');
    });

    it('应该拒绝过长的工作区 ID', () => {
      const longId = 'a'.repeat(51);
      const result = validateWorkspaceIdFrontend(longId);

      expect(result.isValid).toBe(false);
      expect(result.errorMessage).toContain('不能超过');
    });

    it('应该拒绝包含大写字母的 ID', () => {
      const result = validateWorkspaceIdFrontend('MyWorkspace');

      expect(result.isValid).toBe(false);
      expect(result.errorMessage).toContain('只能包含小写字母');
    });

    it('应该拒绝以数字开头的 ID', () => {
      const result = validateWorkspaceIdFrontend('1workspace');

      expect(result.isValid).toBe(false);
      expect(result.errorMessage).toContain('不能以数字开头');
    });

    it('应该拒绝包含连续下划线的 ID', () => {
      const result = validateWorkspaceIdFrontend('my__workspace');

      expect(result.isValid).toBe(false);
      expect(result.errorMessage).toContain('不能包含连续下划线');
    });
  });

  describe('validatePathFrontend', () => {
    it('应该接受有效路径', () => {
      const result = validatePathFrontend('/home/user/documents');

      expect(result.isValid).toBe(true);
    });

    it('应该拒绝空路径', () => {
      const result = validatePathFrontend('');

      expect(result.isValid).toBe(false);
      expect(result.errorMessage).toBe('路径不能为空');
    });

    it('应该拒绝过长的路径', () => {
      const longPath = 'a'.repeat(261);
      const result = validatePathFrontend(longPath);

      expect(result.isValid).toBe(false);
      expect(result.errorMessage).toContain('路径长度');
    });

    it('应该检测父目录引用', () => {
      const result = validatePathFrontend('/home/../../../etc/passwd');

      expect(result.isValid).toBe(false);
      expect(result.errorMessage).toContain('..');
    });

    it('应该检测空字节', () => {
      const result = validatePathFrontend('/home/\x00/user');

      expect(result.isValid).toBe(false);
      expect(result.errorMessage).toContain('禁止的模式');
    });
  });

  describe('validateFilenameFrontend', () => {
    it('应该接受有效文件名', () => {
      const result = validateFilenameFrontend('document.txt');

      expect(result.isValid).toBe(true);
    });

    it('应该拒绝空文件名', () => {
      const result = validateFilenameFrontend('');

      expect(result.isValid).toBe(false);
      expect(result.errorMessage).toBe('文件名不能为空');
    });

    it('应该拒绝包含非法字符的文件名', () => {
      const result = validateFilenameFrontend('file<name>.txt');

      expect(result.isValid).toBe(false);
      expect(result.errorMessage).toBe('文件名包含非法字符');
    });

    it('应该拒绝 Windows 保留名称', () => {
      const result = validateFilenameFrontend('CON.txt');

      expect(result.isValid).toBe(false);
      expect(result.errorMessage).toContain('系统保留名称');
    });

    it('应该拒绝 PRN（大写）', () => {
      const result = validateFilenameFrontend('PRN');

      expect(result.isValid).toBe(false);
      expect(result.errorMessage).toContain('系统保留名称');
    });
  });
});

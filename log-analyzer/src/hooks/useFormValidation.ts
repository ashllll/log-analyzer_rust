/**
 * 表单验证 Hook
 *
 * 使用后端验证命令进行实时表单验证
 *
 * @module hooks/useFormValidation
 */

import { useState, useCallback } from 'react';
import { invoke } from '@tauri-apps/api/core';

// ============================================================================
// 类型定义
// ============================================================================

/**
 * 验证结果
 */
export interface ValidationResult {
  /** 是否验证通过 */
  isValid: boolean;

  /** 错误消息 */
  errorMessage?: string;

  /** 警告消息 */
  warningMessage?: string;

  /** 验证的问题列表 */
  issues?: ValidationIssue[];
}

/**
 * 验证问题
 */
export interface ValidationIssue {
  /** 字段名 */
  field?: string;

  /** 严重级别 */
  severity: 'error' | 'warning' | 'info';

  /** 错误码 */
  code: string;

  /** 消息 */
  message: string;
}

/**
 * 工作区 ID 验证结果
 */
export interface WorkspaceIdValidation {
  /** 是否有效 */
  isValid: boolean;

  /** 错误消息 */
  errorMessage?: string;
}

/**
 * 路径安全验证结果
 */
export interface PathSecurityValidation {
  /** 是否安全 */
  isSafe: boolean;

  /** 验证的问题列表 */
  issues: ValidationIssue[];
}

// ============================================================================
// 验证 Hook
// ============================================================================

/**
 * 表单验证 Hook
 *
 * 提供实时验证功能，调用后端验证命令
 *
 * @example
 * ```typescript
 * const { validateWorkspaceId, validatePathSecurity, isValidating } = useFormValidation();
 *
 * const handleWorkspaceIdChange = async (id: string) => {
 *   const result = await validateWorkspaceId(id);
 *   if (!result.isValid) {
 *     setError(result.errorMessage);
 *   }
 * };
 * ```
 */
export function useFormValidation() {
  const [isValidating, setIsValidating] = useState(false);

  /**
   * 验证工作区 ID 格式
   *
   * @param workspaceId - 工作区 ID
   * @returns 验证结果
   */
  const validateWorkspaceId = useCallback(async (
    workspaceId: string
  ): Promise<WorkspaceIdValidation> => {
    if (!workspaceId || workspaceId.trim().length === 0) {
      return {
        isValid: false,
        errorMessage: '工作区 ID 不能为空',
      };
    }

    setIsValidating(true);

    try {
      const isValid = await invoke<boolean>('validate_workspace_id_format', {
        workspaceId,
      });

      return {
        isValid,
        errorMessage: isValid ? undefined : '工作区 ID 格式无效',
      };
    } catch (error) {
      return {
        isValid: false,
        errorMessage: `验证失败：${error}`,
      };
    } finally {
      setIsValidating(false);
    }
  }, []);

  /**
   * 验证路径安全性
   *
   * @param path - 文件路径
   * @returns 验证结果
   */
  const validatePathSecurity = useCallback(
    async (path: string): Promise<PathSecurityValidation> => {
      if (!path || path.trim().length === 0) {
        return {
          isSafe: false,
          issues: [
            {
              severity: 'error',
              code: 'EMPTY_PATH',
              message: '路径不能为空',
            },
          ],
        };
      }

      setIsValidating(true);

      try {
        const result = await invoke<{
          isSafe: boolean;
          issues: ValidationIssue[];
        }>('validate_path_security', { path });

        return result;
      } catch (error) {
        return {
          isSafe: false,
          issues: [
            {
              severity: 'error',
              code: 'VALIDATION_FAILED',
              message: `验证失败：${error}`,
            },
          ],
        };
      } finally {
        setIsValidating(false);
      }
    },
    []
  );

  /**
   * 验证工作区配置
   *
   * @param workspaceId - 工作区 ID
   * @returns 验证结果
   */
  const validateWorkspaceConfig = useCallback(
    async (workspaceId: string): Promise<ValidationResult> => {
      setIsValidating(true);

      try {
        const result = await invoke<{
          isValid: boolean;
          issues: ValidationIssue[];
        }>('validate_workspace_config_cmd', { workspaceId });

        const errorIssues = result.issues.filter((i) => i.severity === 'error');
        const warningIssues = result.issues.filter((i) => i.severity === 'warning');

        return {
          isValid: result.isValid,
          errorMessage: errorIssues.length > 0 ? errorIssues[0].message : undefined,
          warningMessage: warningIssues.length > 0 ? warningIssues[0].message : undefined,
          issues: result.issues,
        };
      } catch (error) {
        return {
          isValid: false,
          errorMessage: `验证失败：${error}`,
        };
      } finally {
        setIsValidating(false);
      }
    },
    []
  );

  return {
    isValidating,
    validateWorkspaceId,
    validatePathSecurity,
    validateWorkspaceConfig,
  };
}

// ============================================================================
// 验证规则常量
// ============================================================================

/**
 * 工作区 ID 验证规则
 */
export const WORKSPACE_ID_RULES = {
  /** 最小长度 */
  MIN_LENGTH: 3,

  /** 最大长度 */
  MAX_LENGTH: 50,

  /** 允许的字符模式 */
  PATTERN: /^[a-z0-9_-]+$/,

  /** 不能以数字开头 */
  NO_LEADING_DIGIT: true,

  /** 不能使用连续下划线 */
  NO_CONSECUTIVE_UNDERSCORES: true,
} as const;

/**
 * 路径验证规则
 */
export const PATH_RULES = {
  /** 最大路径长度 */
  MAX_LENGTH: 260,

  /** 禁止的路径模式 */
  FORBIDDEN_PATTERNS: [
    '..',           // 父目录引用
    '~',            // 用户目录（在某些情况下）
    'null',         // null 字节
    '\x00',         // null 字符
    'CON',          // Windows 保留名
    'PRN',
    'AUX',
    'NUL',
    'COM[1-9]',     // COM 端口
    'LPT[1-9]',     // LPT 端口
  ],

  /** 允许的协议 */
  ALLOWED_PROTOCOLS: ['file://', ''],
} as const;

/**
 * 文件名验证规则
 */
export const FILENAME_RULES = {
  /** 禁止的字符 */
  FORBIDDEN_CHARS: /[<>:"|?*]/,

  /** 禁止的文件名（Windows） */
  FORBIDDEN_NAMES: [
    'CON', 'PRN', 'AUX', 'NUL',
    'COM1', 'COM2', 'COM3', 'COM4', 'COM5', 'COM6', 'COM7', 'COM8', 'COM9',
    'LPT1', 'LPT2', 'LPT3', 'LPT4', 'LPT5', 'LPT6', 'LPT7', 'LPT8', 'LPT9',
  ] as const,
} as const;

// ============================================================================
// 前端验证函数
// ============================================================================

/**
 * 前端工作区 ID 验证（快速检查）
 *
 * @param workspaceId - 工作区 ID
 * @returns 验证结果
 */
export function validateWorkspaceIdFrontend(workspaceId: string): ValidationResult {
  if (!workspaceId) {
    return { isValid: false, errorMessage: '工作区 ID 不能为空' };
  }

  if (workspaceId.length < WORKSPACE_ID_RULES.MIN_LENGTH) {
    return {
      isValid: false,
      errorMessage: `工作区 ID 至少需要 ${WORKSPACE_ID_RULES.MIN_LENGTH} 个字符`,
    };
  }

  if (workspaceId.length > WORKSPACE_ID_RULES.MAX_LENGTH) {
    return {
      isValid: false,
      errorMessage: `工作区 ID 不能超过 ${WORKSPACE_ID_RULES.MAX_LENGTH} 个字符`,
    };
  }

  if (!WORKSPACE_ID_RULES.PATTERN.test(workspaceId)) {
    return {
      isValid: false,
      errorMessage: '工作区 ID 只能包含小写字母、数字、下划线和连字符',
    };
  }

  if (WORKSPACE_ID_RULES.NO_LEADING_DIGIT && /^\d/.test(workspaceId)) {
    return {
      isValid: false,
      errorMessage: '工作区 ID 不能以数字开头',
    };
  }

  if (WORKSPACE_ID_RULES.NO_CONSECUTIVE_UNDERSCORES && /__/.test(workspaceId)) {
    return {
      isValid: false,
      errorMessage: '工作区 ID 不能包含连续下划线',
    };
  }

  return { isValid: true };
}

/**
 * 前端路径安全验证（快速检查）
 *
 * @param path - 文件路径
 * @returns 验证结果
 */
export function validatePathFrontend(path: string): ValidationResult {
  if (!path) {
    return { isValid: false, errorMessage: '路径不能为空' };
  }

  if (path.length > PATH_RULES.MAX_LENGTH) {
    return {
      isValid: false,
      errorMessage: `路径长度不能超过 ${PATH_RULES.MAX_LENGTH} 个字符`,
    };
  }

  // 检查禁止的模式
  for (const pattern of PATH_RULES.FORBIDDEN_PATTERNS) {
    if (path.includes(pattern)) {
      return {
        isValid: false,
        errorMessage: `路径包含禁止的模式：${pattern}`,
      };
    }
  }

  return { isValid: true };
}

/**
 * 前端文件名验证（快速检查）
 *
 * @param filename - 文件名
 * @returns 验证结果
 */
export function validateFilenameFrontend(filename: string): ValidationResult {
  if (!filename) {
    return { isValid: false, errorMessage: '文件名不能为空' };
  }

  // 检查禁止的字符
  if (FILENAME_RULES.FORBIDDEN_CHARS.test(filename)) {
    return {
      isValid: false,
      errorMessage: '文件名包含非法字符',
    };
  }

  // 检查禁止的名称
  const nameWithoutExt = filename.replace(/\.[^.]+$/, '');
  const upperName = nameWithoutExt.toUpperCase();
  if (FILENAME_RULES.FORBIDDEN_NAMES.includes(upperName as any)) {
    return {
      isValid: false,
      errorMessage: `"${nameWithoutExt}" 是系统保留名称，不能用作文件名`,
    };
  }

  return { isValid: true };
}

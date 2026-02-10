/**
 * 关键词组验证模式
 *
 * 使用 Zod 进行类型安全的表单验证
 * 符合项目"必须使用业内成熟方案"原则
 */

import { z } from 'zod';

/**
 * 单个关键词模式验证
 */
export const keywordPatternSchema = z.object({
  regex: z.string(),
  comment: z.string(),
});

/**
 * 关键词组表单验证模式
 *
 * 验证规则：
 * - name: 必填，2-50个字符，自动trim
 * - color: 必须是预定义的颜色之一
 * - patterns: 至少一个非空正则表达式模式
 *           不能有重复的模式
 *           正则表达式语法必须有效
 */
export const keywordGroupSchema = z.object({
  name: z.string()
    .min(2, '关键词组名称至少需要 2 个字符')
    .max(50, '关键词组名称不能超过 50 个字符')
    .trim(),

  color: z.enum(['blue', 'green', 'orange', 'red', 'purple']),

  patterns: z.array(keywordPatternSchema)
    .min(1, '至少需要一个正则表达式模式')
    // 检查至少有一个非空模式
    .refine(
      (patterns) => patterns.some(p => p.regex.trim()),
      { message: '至少需要一个有效的正则表达式模式' }
    )
    // 检查重复模式
    .refine(
      (patterns) => {
        const regexes = patterns
          .map(p => p.regex.trim())
          .filter(Boolean);
        return new Set(regexes).size === regexes.length;
      },
      { message: '存在重复的正则表达式模式' }
    )
    // 验证每个正则表达式语法
    .refine(
      (patterns) => {
        return patterns.every(p => {
          if (!p.regex.trim()) return true; // 空值跳过
          try {
            new RegExp(p.regex);
            return true;
          } catch {
            return false;
          }
        });
      },
      { message: '存在无效的正则表达式语法' }
    ),
});

/**
 * 关键词组表单数据类型
 * 由 Zod 自动推导，确保类型安全
 */
export type KeywordGroupFormData = z.infer<typeof keywordGroupSchema>;

/**
 * 验证单个正则表达式语法
 *
 * @param regex - 待验证的正则表达式字符串
 * @returns 验证结果和可能的错误信息
 */
export function validateRegexPattern(regex: string): {
  valid: boolean;
  error?: string;
} {
  if (!regex.trim()) {
    return { valid: true }; // 空值视为有效
  }

  try {
    new RegExp(regex);
    return { valid: true };
  } catch (error) {
    return {
      valid: false,
      error: error instanceof Error ? error.message : '无效的正则表达式语法',
    };
  }
}

/**
 * 验证关键词组表单数据
 *
 * @param data - 待验证的表单数据
 * @returns 验证结果
 */
export function validateKeywordGroup(data: unknown) {
  return keywordGroupSchema.safeParse(data);
}

/**
 * 错误消息类型
 */
export interface FormattedErrors {
  name?: string;
  patterns?: string[];
  color?: string;
}

/**
 * 获取格式化的错误消息
 *
 * @param result - Zod 验证结果（可能是成功或失败）
 * @returns 用户友好的错误消息对象
 */
export function formatValidationErrors(
  result: ReturnType<typeof validateKeywordGroup>
): FormattedErrors {
  const errors: FormattedErrors = {};

  if (!result.success) {
    const formatted = result.error.format();

    if (formatted.name?._errors?.[0]) {
      errors.name = formatted.name._errors[0];
    }

    if (formatted.color?._errors?.[0]) {
      errors.color = formatted.color._errors[0];
    }

    if (formatted.patterns) {
      const patternErrors: string[] = [];

      // 处理数组级别的错误
      if (formatted.patterns._errors?.length) {
        formatted.patterns._errors.forEach((err: unknown) => {
          patternErrors.push(String(err));
        });
      }

      // 处理每个模式的错误
      Object.entries(formatted.patterns).forEach(([key, value]) => {
        if (key === '_errors') return;

        const index = parseInt(key, 10);
        if (!isNaN(index) && value && typeof value === 'object') {
          const patternValue = value as Record<string, unknown>;

          // 处理字段级别的错误
          Object.entries(patternValue).forEach(([fieldKey, fieldValue]) => {
            if (fieldKey === '_errors') {
              const errorArray = fieldValue as { _errors?: string[] };
              if (errorArray._errors?.length) {
                patternErrors[index] = errorArray._errors[0];
              }
            } else if (fieldKey === 'regex' || fieldKey === 'comment') {
              const fieldErrors = fieldValue as { _errors?: string[] };
              if (fieldErrors._errors?.length) {
                patternErrors[index] = `第 ${index + 1} 个${fieldKey === 'regex' ? '模式' : '注释'}: ${fieldErrors._errors[0]}`;
              }
            }
          });
        }
      });

      if (patternErrors.length > 0) {
        errors.patterns = patternErrors;
      }
    }
  }

  return errors;
}

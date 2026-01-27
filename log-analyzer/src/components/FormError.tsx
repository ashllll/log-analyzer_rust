import React from 'react';

interface FormErrorProps {
  /**
   * 错误消息
   */
  message?: string;
  /**
   * 字段 ID，用于无障碍关联
   */
  fieldId?: string;
  /**
   * 是否显示错误图标
   */
  showIcon?: boolean;
  /**
   * 自定义类名
   */
  className?: string;
}

/**
 * 表单错误组件 - 符合无障碍标准
 * 
 * 特性：
 * - ARIA 标签支持
 * - 屏幕阅读器友好
 * - 视觉错误指示
 * - 自动关联到表单字段
 */
export const FormError: React.FC<FormErrorProps> = ({
  message,
  fieldId,
  showIcon = true,
  className = '',
}) => {
  if (!message) {
    return null;
  }

  const errorId = fieldId ? `${fieldId}-error` : undefined;

  return (
    <div
      id={errorId}
      role="alert"
      aria-live="polite"
      className={`flex items-start gap-2 mt-1 text-sm text-red-600 dark:text-red-400 ${className}`}
    >
      {showIcon && (
        <svg
          className="w-4 h-4 flex-shrink-0 mt-0.5"
          fill="none"
          stroke="currentColor"
          viewBox="0 0 24 24"
          aria-hidden="true"
        >
          <path
            strokeLinecap="round"
            strokeLinejoin="round"
            strokeWidth={2}
            d="M12 8v4m0 4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z"
          />
        </svg>
      )}
      <span>{message}</span>
    </div>
  );
};

/**
 * 表单字段包装器 - 自动处理错误显示和无障碍属性
 * 注意：这是旧版本的 FormField，新代码应使用 components/ui/FormField
 * @deprecated 使用 components/ui/FormField 替代
 */
interface FormFieldProps {
  /**
   * 字段 ID
   */
  id: string;
  /**
   * 字段标签
   */
  label: string;
  /**
   * 错误消息
   */
  error?: string;
  /**
   * 是否必填
   */
  required?: boolean;
  /**
   * 子元素（输入框）
   */
  children: React.ReactNode;
  /**
   * 帮助文本
   */
  helpText?: string;
  /**
   * 自定义类名
   */
  className?: string;
}

/**
 * @deprecated 使用 components/ui/FormField 替代
 */
export const LegacyFormField: React.FC<FormFieldProps> = ({
  id,
  label,
  error,
  required = false,
  children,
  helpText,
  className = '',
}) => {
  const errorId = `${id}-error`;
  const helpId = `${id}-help`;
  const hasError = Boolean(error);

  return (
    <div className={`space-y-1 ${className}`}>
      <label
        htmlFor={id}
        className="block text-sm font-medium text-gray-700 dark:text-gray-300"
      >
        {label}
        {required && (
          <span className="text-red-600 dark:text-red-400 ml-1" aria-label="必填">
            *
          </span>
        )}
      </label>

      <div className="relative">
        {React.Children.map(children, (child) => {
          if (React.isValidElement(child)) {
            return React.cloneElement(child as React.ReactElement<any>, {
              id,
              'aria-invalid': hasError,
              'aria-describedby': [
                hasError ? errorId : null,
                helpText ? helpId : null,
              ]
                .filter(Boolean)
                .join(' ') || undefined,
            });
          }
          return child;
        })}
      </div>

      {helpText && !hasError && (
        <p
          id={helpId}
          className="text-xs text-gray-500 dark:text-gray-400"
        >
          {helpText}
        </p>
      )}

      <FormError message={error} fieldId={id} />
    </div>
  );
};

/**
 * 表单错误摘要 - 显示所有表单错误
 * 
 * 用于在表单顶部显示所有验证错误的摘要
 */
interface FormErrorSummaryProps {
  /**
   * 错误列表
   */
  errors: Record<string, string>;
  /**
   * 标题
   */
  title?: string;
  /**
   * 自定义类名
   */
  className?: string;
}

export const FormErrorSummary: React.FC<FormErrorSummaryProps> = ({
  errors,
  title = '请修正以下错误',
  className = '',
}) => {
  const errorEntries = Object.entries(errors).filter(([, message]) => message);

  if (errorEntries.length === 0) {
    return null;
  }

  return (
    <div
      role="alert"
      aria-live="polite"
      className={`p-4 bg-red-50 dark:bg-red-900/20 border border-red-200 dark:border-red-800 rounded-lg ${className}`}
    >
      <div className="flex items-start gap-3">
        <svg
          className="w-5 h-5 text-red-600 dark:text-red-400 flex-shrink-0 mt-0.5"
          fill="none"
          stroke="currentColor"
          viewBox="0 0 24 24"
          aria-hidden="true"
        >
          <path
            strokeLinecap="round"
            strokeLinejoin="round"
            strokeWidth={2}
            d="M12 8v4m0 4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z"
          />
        </svg>
        <div className="flex-1 min-w-0">
          <h3 className="text-sm font-medium text-red-800 dark:text-red-300 mb-2">
            {title}
          </h3>
          <ul className="list-disc list-inside space-y-1">
            {errorEntries.map(([field, message]) => (
              <li
                key={field}
                className="text-sm text-red-700 dark:text-red-400"
              >
                {message}
              </li>
            ))}
          </ul>
        </div>
      </div>
    </div>
  );
};

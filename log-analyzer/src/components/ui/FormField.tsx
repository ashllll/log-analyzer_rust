import React from 'react';
import { cn } from '../../utils/classNames';

interface FormFieldProps {
  children: React.ReactNode;
  label?: string;
  error?: string | string[];
  required?: boolean;
  className?: string;
  id?: string;
  description?: string;
}

/**
 * Accessible form field wrapper with error display
 */
export const FormField: React.FC<FormFieldProps> = ({
  children,
  label,
  error,
  required = false,
  className,
  id,
  description
}) => {
  const fieldId = id || `field-${Math.random().toString(36).substr(2, 9)}`;
  const errorId = `${fieldId}-error`;
  const descriptionId = `${fieldId}-description`;
  const hasError = !!error;
  const errorMessages = Array.isArray(error) ? error : error ? [error] : [];

  return (
    <div className={cn('space-y-2', className)}>
      {/* Label */}
      {label && (
        <label 
          htmlFor={fieldId}
          className={cn(
            'block text-sm font-medium',
            hasError ? 'text-red-600 dark:text-red-400' : 'text-text-main'
          )}
        >
          {label}
          {required && (
            <span 
              className="text-red-500 ml-1" 
              aria-label="required"
            >
              *
            </span>
          )}
        </label>
      )}

      {/* Description */}
      {description && (
        <p 
          id={descriptionId}
          className="text-sm text-text-muted"
        >
          {description}
        </p>
      )}

      {/* Input Field */}
      <div className="relative">
        {React.Children.map(children, child => {
          if (!React.isValidElement(child)) return child;
          
          // 仅对 Input 或类似交互组件注入属性
          const isInput = typeof child.type === 'string' || 
                         (child.type as any).displayName === 'Input' ||
                         (child.type as any).name === 'Input';

          if (isInput) {
            return React.cloneElement(child as React.ReactElement<any>, {
              id: fieldId,
              'aria-invalid': hasError,
              'aria-describedby': cn(
                description && descriptionId,
                hasError && errorId
              ).trim() || undefined,
              className: cn(
                (child as React.ReactElement<any>).props?.className,
                hasError && 'border-red-500 focus:border-red-500 focus:ring-red-500'
              )
            });
          }
          return child;
        })}
      </div>

      {/* Error Messages */}
      {hasError && (
        <div 
          id={errorId}
          role="alert"
          aria-live="polite"
          className="space-y-1"
        >
          {errorMessages.map((message, index) => (
            <p 
              key={index}
              className="text-sm text-red-600 dark:text-red-400 flex items-start gap-2"
            >
              <svg
                className="h-4 w-4 mt-0.5 flex-shrink-0"
                fill="currentColor"
                viewBox="0 0 20 20"
                aria-hidden="true"
              >
                <path
                  fillRule="evenodd"
                  d="M18 10a8 8 0 11-16 0 8 8 0 0116 0zm-7 4a1 1 0 11-2 0 1 1 0 012 0zm-1-9a1 1 0 00-1 1v4a1 1 0 102 0V6a1 1 0 00-1-1z"
                  clipRule="evenodd"
                />
              </svg>
              <span>{message}</span>
            </p>
          ))}
        </div>
      )}
    </div>
  );
};

/**
 * Form group for organizing multiple fields
 */
export const FormGroup: React.FC<{
  children: React.ReactNode;
  title?: string;
  description?: string;
  className?: string;
}> = ({ children, title, description, className }) => {
  return (
    <fieldset className={cn('space-y-4', className)}>
      {title && (
        <legend className="text-lg font-medium text-text-main mb-2">
          {title}
        </legend>
      )}
      {description && (
        <p className="text-sm text-text-muted mb-4">
          {description}
        </p>
      )}
      <div className="space-y-4">
        {children}
      </div>
    </fieldset>
  );
};

/**
 * Form error summary for accessibility
 */
export const FormErrorSummary: React.FC<{
  errors: Record<string, string | string[]>;
  title?: string;
  className?: string;
}> = ({ errors, title = "Please correct the following errors:", className }) => {
  const errorEntries = Object.entries(errors).filter(([, error]) => error);
  
  if (errorEntries.length === 0) {
    return null;
  }

  return (
    <div 
      role="alert"
      aria-live="polite"
      className={cn(
        'bg-red-50 dark:bg-red-900/20 border border-red-200 dark:border-red-800 rounded-md p-4',
        className
      )}
    >
      <div className="flex">
        <div className="flex-shrink-0">
          <svg
            className="h-5 w-5 text-red-400"
            fill="currentColor"
            viewBox="0 0 20 20"
            aria-hidden="true"
          >
            <path
              fillRule="evenodd"
              d="M18 10a8 8 0 11-16 0 8 8 0 0116 0zm-7 4a1 1 0 11-2 0 1 1 0 012 0zm-1-9a1 1 0 00-1 1v4a1 1 0 102 0V6a1 1 0 00-1-1z"
              clipRule="evenodd"
            />
          </svg>
        </div>
        <div className="ml-3">
          <h3 className="text-sm font-medium text-red-800 dark:text-red-200">
            {title}
          </h3>
          <div className="mt-2 text-sm text-red-700 dark:text-red-300">
            <ul className="list-disc list-inside space-y-1">
              {errorEntries.map(([field, error]) => {
                const messages = Array.isArray(error) ? error : [error];
                return messages.map((message, index) => (
                  <li key={`${field}-${index}`}>
                    <span className="font-medium capitalize">{field.replace(/([A-Z])/g, ' $1').trim()}:</span> {message}
                  </li>
                ));
              })}
            </ul>
          </div>
        </div>
      </div>
    </div>
  );
};
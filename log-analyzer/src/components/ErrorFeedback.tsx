import React, { useState } from 'react';
import { Button } from './ui';

interface ErrorFeedbackProps {
  /**
   * 错误信息
   */
  error: Error;
  /**
   * 错误上下文
   */
  context?: Record<string, any>;
  /**
   * 提交反馈的回调
   */
  onSubmit?: (feedback: ErrorFeedback) => Promise<void>;
  /**
   * 关闭回调
   */
  onClose?: () => void;
}

export interface ErrorFeedback {
  error: {
    message: string;
    stack?: string;
  };
  context?: Record<string, any>;
  userDescription: string;
  userEmail?: string;
  timestamp: string;
}

/**
 * 错误反馈组件 - 收集用户对错误的反馈
 * 
 * 用于在错误发生时收集用户的描述和联系方式
 */
export const ErrorFeedbackForm: React.FC<ErrorFeedbackProps> = ({
  error,
  context,
  onSubmit,
  onClose,
}) => {
  const [description, setDescription] = useState('');
  const [email, setEmail] = useState('');
  const [isSubmitting, setIsSubmitting] = useState(false);
  const [submitted, setSubmitted] = useState(false);

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();

    if (!description.trim()) {
      return;
    }

    setIsSubmitting(true);

    try {
      const feedback: ErrorFeedback = {
        error: {
          message: error.message,
          stack: error.stack,
        },
        context,
        userDescription: description,
        userEmail: email || undefined,
        timestamp: new Date().toISOString(),
      };

      if (onSubmit) {
        await onSubmit(feedback);
      } else {
        // 默认行为：记录到控制台
        console.log('Error feedback:', feedback);
      }

      setSubmitted(true);
    } catch (err) {
      console.error('Failed to submit feedback:', err);
    } finally {
      setIsSubmitting(false);
    }
  };

  if (submitted) {
    return (
      <div className="p-6 bg-white dark:bg-gray-800 rounded-lg shadow-lg max-w-md">
        <div className="text-center space-y-4">
          <div className="w-12 h-12 bg-green-100 dark:bg-green-900/30 rounded-full flex items-center justify-center mx-auto">
            <svg
              className="w-6 h-6 text-green-600 dark:text-green-400"
              fill="none"
              stroke="currentColor"
              viewBox="0 0 24 24"
            >
              <path
                strokeLinecap="round"
                strokeLinejoin="round"
                strokeWidth={2}
                d="M5 13l4 4L19 7"
              />
            </svg>
          </div>
          <div>
            <h3 className="text-lg font-medium text-gray-900 dark:text-gray-100">
              感谢您的反馈
            </h3>
            <p className="text-sm text-gray-600 dark:text-gray-400 mt-2">
              我们已收到您的反馈，将尽快处理此问题。
            </p>
          </div>
          <Button onClick={onClose} className="w-full">
            关闭
          </Button>
        </div>
      </div>
    );
  }

  return (
    <div className="p-6 bg-white dark:bg-gray-800 rounded-lg shadow-lg max-w-md">
      <h3 className="text-lg font-medium text-gray-900 dark:text-gray-100 mb-4">
        帮助我们改进
      </h3>
      <p className="text-sm text-gray-600 dark:text-gray-400 mb-4">
        如果您愿意，请描述一下发生了什么。这将帮助我们更快地解决问题。
      </p>

      <form onSubmit={handleSubmit} className="space-y-4">
        {/* 错误描述 */}
        <div>
          <label
            htmlFor="error-description"
            className="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1"
          >
            发生了什么？ <span className="text-red-600">*</span>
          </label>
          <textarea
            id="error-description"
            value={description}
            onChange={(e) => setDescription(e.target.value)}
            placeholder="请描述您在做什么时遇到了这个错误..."
            rows={4}
            required
            className="w-full px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-gray-700 text-gray-900 dark:text-gray-100 placeholder-gray-400 dark:placeholder-gray-500 focus:outline-none focus:ring-2 focus:ring-blue-500 dark:focus:ring-blue-400"
          />
        </div>

        {/* 联系邮箱（可选） */}
        <div>
          <label
            htmlFor="error-email"
            className="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1"
          >
            联系邮箱（可选）
          </label>
          <input
            id="error-email"
            type="email"
            value={email}
            onChange={(e) => setEmail(e.target.value)}
            placeholder="your@email.com"
            className="w-full px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-gray-700 text-gray-900 dark:text-gray-100 placeholder-gray-400 dark:placeholder-gray-500 focus:outline-none focus:ring-2 focus:ring-blue-500 dark:focus:ring-blue-400"
          />
          <p className="text-xs text-gray-500 dark:text-gray-400 mt-1">
            如果您希望我们在问题解决后通知您，请留下邮箱
          </p>
        </div>

        {/* 按钮 */}
        <div className="flex gap-3">
          <Button
            type="submit"
            disabled={isSubmitting || !description.trim()}
            className="flex-1 bg-blue-600 hover:bg-blue-700 text-white disabled:opacity-50 disabled:cursor-not-allowed"
          >
            {isSubmitting ? (
              <>
                <svg
                  className="animate-spin w-4 h-4 mr-2"
                  fill="none"
                  viewBox="0 0 24 24"
                >
                  <circle
                    className="opacity-25"
                    cx="12"
                    cy="12"
                    r="10"
                    stroke="currentColor"
                    strokeWidth="4"
                  />
                  <path
                    className="opacity-75"
                    fill="currentColor"
                    d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"
                  />
                </svg>
                提交中...
              </>
            ) : (
              '提交反馈'
            )}
          </Button>
          {onClose && (
            <Button
              type="button"
              onClick={onClose}
              className="flex-1 bg-gray-600 hover:bg-gray-700 text-white"
            >
              跳过
            </Button>
          )}
        </div>
      </form>
    </div>
  );
};

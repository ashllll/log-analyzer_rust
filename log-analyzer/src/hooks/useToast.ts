/**
 * 统一的 Toast Hook - 基于 react-hot-toast
 * 
 * 提供业内成熟的通知方案，具备以下特性：
 * - 自动消失（可配置时长）
 * - 流畅动画
 * - 可堆叠显示
 * - 支持自定义样式
 * - 可手动关闭
 */

import toast from 'react-hot-toast';

export type ToastType = 'success' | 'error' | 'info' | 'loading';

interface ToastOptions {
  duration?: number;
  position?: 'top-left' | 'top-center' | 'top-right' | 'bottom-left' | 'bottom-center' | 'bottom-right';
}

/**
 * Toast Hook
 * 
 * @example
 * const { showToast } = useToast();
 * showToast('success', '操作成功');
 * showToast('error', '操作失败', { duration: 5000 });
 */
export const useToast = () => {
  const showToast = (type: ToastType, message: string, options?: ToastOptions) => {
    const defaultOptions = {
      duration: type === 'error' ? 4000 : 3000,
      position: 'bottom-right' as const,
      ...options,
    };

    switch (type) {
      case 'success':
        return toast.success(message, defaultOptions);
      case 'error':
        return toast.error(message, defaultOptions);
      case 'info':
        return toast(message, {
          ...defaultOptions,
          icon: 'ℹ️',
        });
      case 'loading':
        return toast.loading(message, defaultOptions);
      default:
        return toast(message, defaultOptions);
    }
  };

  const dismissToast = (toastId?: string) => {
    if (toastId) {
      toast.dismiss(toastId);
    } else {
      toast.dismiss();
    }
  };

  const promise = <T,>(
    promise: Promise<T>,
    messages: {
      loading: string;
      success: string | ((data: T) => string);
      error: string | ((error: Error) => string);
    },
    options?: ToastOptions
  ) => {
    return toast.promise(promise, messages, options);
  };

  return {
    showToast,
    dismissToast,
    promise,
  };
};

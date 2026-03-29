/**
 * 统一 Toast Hook - 基于 react-hot-toast
 *
 * 全项目唯一的 Toast 管理入口，提供以下特性：
 * - 自动消失（可配置时长）
 * - 流畅动画
 * - 可堆叠显示
 * - 支持自定义样式
 * - 可手动关闭
 *
 * @example
 * const { showToast, showSuccess, showError } = useToast();
 * showToast('success', '操作成功');
 * showSuccess('操作成功');
 * showError('操作失败', { duration: 5000 });
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
 * 所有组件应通过此 Hook 显示 Toast 通知，不要直接使用 react-hot-toast 或 appStore.addToast。
 */
export const useToast = () => {
  const showToast = (type: ToastType, message: string, options?: number | ToastOptions) => {
    // 兼容旧 API：第三个参数可能是 number（duration）或 ToastOptions
    const resolvedOptions: ToastOptions = typeof options === 'number'
      ? { duration: options }
      : options ?? {};

    const defaultOptions = {
      duration: type === 'error' ? 4000 : 3000,
      position: 'bottom-right' as const,
      ...resolvedOptions,
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

  // 便捷方法：显示成功提示
  const showSuccess = (message: string, options?: number | ToastOptions) => {
    return showToast('success', message, options);
  };

  // 便捷方法：显示错误提示
  const showError = (message: string, options?: number | ToastOptions) => {
    return showToast('error', message, options);
  };

  // 便捷方法：显示信息提示
  const showInfo = (message: string, options?: number | ToastOptions) => {
    return showToast('info', message, options);
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
    showSuccess,
    showError,
    showInfo,
    dismissToast,
    promise,
  };
};

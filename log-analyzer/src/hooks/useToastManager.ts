import { useCallback } from 'react';
import toast from 'react-hot-toast';
import type { ToastType } from '../stores/appStore';

/**
 * Hook for managing toast notifications using react-hot-toast
 * This is an industry-standard solution with automatic lifecycle management
 */
export const useToastManager = () => {
  // Show toast with type
  const showToast = useCallback((type: ToastType, message: string, duration = 3000) => {
    const options = { duration };
    
    switch (type) {
      case 'success':
        return toast.success(message, options);
      case 'error':
        return toast.error(message, options);
      case 'info':
        return toast(message, options);
      default:
        return toast(message, options);
    }
  }, []);

  // Show success toast
  const showSuccess = useCallback((message: string, duration?: number) => {
    return toast.success(message, { duration: duration || 3000 });
  }, []);

  // Show error toast
  const showError = useCallback((message: string, duration?: number) => {
    return toast.error(message, { duration: duration || 3000 });
  }, []);

  // Show info toast
  const showInfo = useCallback((message: string, duration?: number) => {
    return toast(message, { duration: duration || 3000 });
  }, []);

  // Manual toast removal
  const dismissToast = useCallback((id: string) => {
    toast.dismiss(id);
  }, []);

  // Dismiss all toasts
  const dismissAll = useCallback(() => {
    toast.dismiss();
  }, []);

  return {
    showToast,
    showSuccess,
    showError,
    showInfo,
    dismissToast,
    dismissAll,
  };
};
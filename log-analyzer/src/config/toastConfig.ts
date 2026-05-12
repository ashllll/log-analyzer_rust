import type { DefaultToastOptions } from 'react-hot-toast';

/**
 * 全局 Toast 样式配置
 * 集中管理所有 Toast 视觉样式，避免在组件中硬编码主题值
 */
export const toastConfig: DefaultToastOptions = {
  duration: 3000,
  style: {
    background: '#27272A', // Zinc-800
    color: '#F4F4F5', // Zinc-100
    border: '1px solid #3F3F46', // Zinc-700
    borderRadius: '0.5rem',
    padding: '0.75rem 1rem',
    fontSize: '0.875rem',
    maxWidth: '400px',
    boxShadow: '0 4px 6px -1px rgb(0 0 0 / 0.3)',
  },
  success: {
    duration: 2500,
    iconTheme: {
      primary: '#10B981', // Emerald-500
      secondary: '#27272A',
    },
    style: {
      border: '1px solid rgba(16, 185, 129, 0.4)',
    },
  },
  error: {
    duration: 4000,
    iconTheme: {
      primary: '#EF4444', // Red-500
      secondary: '#27272A',
    },
    style: {
      border: '1px solid rgba(239, 68, 68, 0.4)',
    },
  },
};

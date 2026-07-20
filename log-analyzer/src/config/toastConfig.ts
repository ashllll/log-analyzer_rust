import type { DefaultToastOptions } from "react-hot-toast";

/**
 * 全局 Toast 样式配置
 * 集中管理所有 Toast 视觉样式，避免在组件中硬编码主题值
 */
export const toastConfig: DefaultToastOptions = {
  duration: 3000,
  style: {
    background: "var(--material-overlay)",
    color: "rgb(var(--color-text-main))",
    border: "1px solid rgb(var(--color-border-base))",
    borderRadius: "0.875rem",
    padding: "0.75rem 1rem",
    fontSize: "0.875rem",
    maxWidth: "400px",
    backdropFilter: "saturate(165%) blur(24px)",
    boxShadow: "0 18px 48px rgb(0 0 0 / 0.28)",
  },
  success: {
    duration: 2500,
    iconTheme: {
      primary: "rgb(var(--color-success))",
      secondary: "rgb(var(--color-bg-card))",
    },
    style: {
      border: "1px solid rgb(var(--color-success) / .4)",
    },
  },
  error: {
    duration: 4000,
    iconTheme: {
      primary: "rgb(var(--color-danger))",
      secondary: "rgb(var(--color-bg-card))",
    },
    style: {
      border: "1px solid rgb(var(--color-danger) / .4)",
    },
  },
};

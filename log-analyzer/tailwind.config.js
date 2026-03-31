/** @type {import('tailwindcss').Config} */
export default {
  content: [
    "./index.html",
    "./src/**/*.{js,ts,jsx,tsx}",
  ],
  theme: {
    extend: {
      colors: {
        // Slate 色系 - 更专业的深色界面，更好的对比度
        bg: {
          main: "#0F172A",    // Slate-900 - 主背景
          sidebar: "#1E293B", // Slate-800 - 侧边栏/面板
          card: "#334155",    // Slate-700 - 卡片/输入框
          hover: "#475569",   // Slate-600 - 悬停态
          elevated: "#1E293B", // 浮动元素背景
          popover: "#18181B",  // Zinc-900 - 弹出层/下拉面板背景
          subtle: "#1E293B",   // Slate-800 - 微妙背景/空状态
          surface: "#334155",  // Slate-700 - 表面背景（按钮、输入框）
        },
        border: {
          base: "#334155",    // Slate-700 - 基础边框
          light: "#475569",   // Slate-600 - 亮边框
          subtle: "#1E293B",  // Slate-800 - 微妙边框
        },
        primary: {
          DEFAULT: "#3B82F6", // Blue-500 - 主色
          hover: "#2563EB",   // Blue-600 - 悬停
          text: "#60A5FA",    // Blue-400 - 文本
          muted: "#93C5FD",   // Blue-300 - 柔和
        },
        // CTA 强调色 - 用于重要操作
        cta: {
          DEFAULT: "#22C55E", // Green-500 - CTA 主色
          hover: "#16A34A",   // Green-600 - 悬停
          text: "#4ADE80",    // Green-400 - 文本
        },
        text: {
          main: "#F1F5F9",    // Slate-100 - 主文本
          muted: "#94A3B8",   // Slate-400 - 次要文本
          dim: "#64748B",     // Slate-500 - 暗淡文本
        },
        // 状态色 - 优化对比度
        status: {
          error: "#EF4444",   // Red-500
          warn: "#F59E0B",    // Amber-500
          info: "#3B82F6",    // Blue-500
          success: "#22C55E", // Green-500
        },
        // 日志级别专用色
        log: {
          error: "#F87171",   // Red-400
          warn: "#FBBF24",    // Amber-400
          info: "#60A5FA",    // Blue-400
          debug: "#A78BFA",   // Violet-400
        }
      },
      fontFamily: {
        sans: ['"Fira Sans"', '"Inter"', '-apple-system', 'BlinkMacSystemFont', 'sans-serif'],
        mono: ['"Fira Code"', '"JetBrains Mono"', '"SF Mono"', 'monospace'],
      },
      fontSize: {
        'xxs': '0.65rem',
      },
      boxShadow: {
        'card': '0 1px 3px 0 rgb(0 0 0 / 0.3), 0 1px 2px -1px rgb(0 0 0 / 0.3)',
        'card-hover': '0 4px 6px -1px rgb(0 0 0 / 0.3), 0 2px 4px -2px rgb(0 0 0 / 0.3)',
        'elevated': '0 10px 15px -3px rgb(0 0 0 / 0.3), 0 4px 6px -4px rgb(0 0 0 / 0.3)',
        'glow-primary': '0 0 20px rgba(59, 130, 246, 0.3)',
        'glow-cta': '0 0 20px rgba(34, 197, 94, 0.3)',
      },
      animation: {
        'fade-in': 'fadeIn 150ms ease-out',
        'slide-in': 'slideIn 200ms ease-out',
        'scale-in': 'scaleIn 150ms ease-out',
      },
      keyframes: {
        fadeIn: {
          '0%': { opacity: '0' },
          '100%': { opacity: '1' },
        },
        slideIn: {
          '0%': { opacity: '0', transform: 'translateY(-10px)' },
          '100%': { opacity: '1', transform: 'translateY(0)' },
        },
        scaleIn: {
          '0%': { opacity: '0', transform: 'scale(0.95)' },
          '100%': { opacity: '1', transform: 'scale(1)' },
        },
      },
    },
  },
  plugins: [],
  // 防止生产构建 PurgeCSS 清除动态颜色类（COLOR_STYLES 通过对象查找动态使用）
  safelist: [
    // highlight: bg-{color}-500/20 text-{color}-300 border-{color}-500/30
    // badge:     bg-{color}-500/15 text-{color}-400 border-{color}-500/20
    { pattern: /^bg-(blue|green|red|orange|purple)-\d+\/(10|15|20|30)$/ },
    { pattern: /^text-(blue|green|red|orange|purple)-\d+$/ },
    { pattern: /^border-(blue|green|red|orange|purple)-\d+\/(10|15|20|30)$/ },
  ],
}
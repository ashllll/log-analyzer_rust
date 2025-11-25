/** @type {import('tailwindcss').Config} */
export default {
  content: [
    "./index.html",
    "./src/**/*.{js,ts,jsx,tsx}",
  ],
  theme: {
    extend: {
      colors: {
        // 使用 Zinc 色系模拟高级深色界面
        bg: {
          main: "#09090b",    // 最深背景 (Main Area)
          sidebar: "#18181b", // 侧边栏/面板 (Zinc-900)
          card: "#27272a",    // 卡片/输入框 (Zinc-800)
          hover: "#3f3f46",   // 悬停态
        },
        border: {
          base: "#27272a",    // 基础边框
          light: "#3f3f46",   // 亮一点的边框
        },
        primary: {
          DEFAULT: "#2563eb", // 截图中的蓝色按钮 (Blue-600)
          hover: "#1d4ed8",   // Blue-700
          text: "#60a5fa",    // Blue-400
        },
        text: {
          main: "#e4e4e7",    // 主文本 (Zinc-200)
          muted: "#a1a1aa",   // 次要文本 (Zinc-400)
          dim: "#71717a",     // 暗淡文本 (Zinc-500)
        },
        // 状态色
        status: {
          error: "#ef4444",
          warn: "#f59e0b",
          info: "#3b82f6",
          success: "#10b981",
        }
      },
      fontFamily: {
        sans: ['"Inter"', '-apple-system', 'BlinkMacSystemFont', 'sans-serif'], // UI 字体
        mono: ['"JetBrains Mono"', '"Fira Code"', 'monospace'], // 代码字体
      },
      fontSize: {
        'xxs': '0.65rem',
      }
    },
  },
  plugins: [],
}
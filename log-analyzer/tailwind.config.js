/** @type {import('tailwindcss').Config} */
export default {
  content: ["./index.html", "./src/**/*.{js,ts,jsx,tsx}"],
  theme: {
    extend: {
      colors: {
        bg: {
          main: "rgb(var(--color-bg-main) / <alpha-value>)",
          sidebar: "rgb(var(--color-bg-sidebar) / <alpha-value>)",
          card: "rgb(var(--color-bg-card) / <alpha-value>)",
          hover: "rgb(var(--color-bg-hover) / <alpha-value>)",
          elevated: "rgb(var(--color-bg-elevated) / <alpha-value>)",
          popover: "rgb(var(--color-bg-popover) / <alpha-value>)",
          subtle: "rgb(var(--color-bg-subtle) / <alpha-value>)",
          surface: "rgb(var(--color-bg-surface) / <alpha-value>)",
        },
        border: {
          base: "rgb(var(--color-border-base) / <alpha-value>)",
          light: "rgb(var(--color-border-light) / <alpha-value>)",
          subtle: "rgb(var(--color-border-subtle) / <alpha-value>)",
        },
        primary: {
          DEFAULT: "rgb(var(--color-accent) / <alpha-value>)",
          hover: "rgb(var(--color-accent-hover) / <alpha-value>)",
          text: "rgb(var(--color-accent) / <alpha-value>)",
          muted: "rgb(var(--color-accent-muted) / <alpha-value>)",
        },
        cta: {
          DEFAULT: "rgb(var(--color-accent) / <alpha-value>)",
          hover: "rgb(var(--color-accent-hover) / <alpha-value>)",
          text: "rgb(var(--color-success) / <alpha-value>)",
        },
        text: {
          main: "rgb(var(--color-text-main) / <alpha-value>)",
          muted: "rgb(var(--color-text-muted) / <alpha-value>)",
          dim: "rgb(var(--color-text-dim) / <alpha-value>)",
        },
        status: {
          error: "rgb(var(--color-danger) / <alpha-value>)",
          warn: "rgb(var(--color-warning) / <alpha-value>)",
          info: "rgb(var(--color-info) / <alpha-value>)",
          success: "rgb(var(--color-success) / <alpha-value>)",
        },
        log: {
          error: "rgb(var(--color-log-error) / <alpha-value>)",
          warn: "rgb(var(--color-log-warn) / <alpha-value>)",
          info: "rgb(var(--color-log-info) / <alpha-value>)",
          debug: "rgb(var(--color-log-debug) / <alpha-value>)",
        },
      },
      fontFamily: {
        sans: [
          "-apple-system",
          "BlinkMacSystemFont",
          '"SF Pro Text"',
          '"Helvetica Neue"',
          "system-ui",
          "sans-serif",
        ],
        mono: [
          '"SFMono-Regular"',
          '"SF Mono"',
          "ui-monospace",
          "Consolas",
          "monospace",
        ],
      },
      fontSize: { xxs: "0.65rem" },
      boxShadow: {
        card: "0 1px 0 rgb(255 255 255 / 0.025)",
        "card-hover": "0 10px 30px rgb(0 0 0 / 0.14)",
        elevated: "0 22px 60px rgb(0 0 0 / 0.34)",
        "glow-primary": "none",
        "glow-cta": "none",
      },
      animation: {
        "fade-in": "fadeIn 150ms var(--ease-out-ui)",
        "slide-in": "slideIn 200ms var(--ease-out-ui)",
        "scale-in": "scaleIn 150ms var(--ease-out-ui)",
      },
      keyframes: {
        fadeIn: { "0%": { opacity: "0" }, "100%": { opacity: "1" } },
        slideIn: {
          "0%": { opacity: "0", transform: "translateY(-10px)" },
          "100%": { opacity: "1", transform: "translateY(0)" },
        },
        scaleIn: {
          "0%": { opacity: "0", transform: "scale(0.95)" },
          "100%": { opacity: "1", transform: "scale(1)" },
        },
      },
    },
  },
  plugins: [],
  safelist: [
    { pattern: /^bg-(blue|green|red|orange|purple)-\d+\/(10|15|20|30)$/ },
    { pattern: /^text-(blue|green|red|orange|purple)-\d+$/ },
    { pattern: /^border-(blue|green|red|orange|purple)-\d+\/(10|15|20|30)$/ },
  ],
};

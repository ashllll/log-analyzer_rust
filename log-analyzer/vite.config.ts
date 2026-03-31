import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";

// @ts-expect-error process is a nodejs global
const host = process.env.TAURI_DEV_HOST;

// https://vite.dev/config/
export default defineConfig(async () => ({
  plugins: [react()],

  // Vite options tailored for Tauri development and only applied in `tauri dev` or `tauri build`
  //
  // 1. prevent Vite from obscuring rust errors
  clearScreen: false,
  // 2. tauri expects a fixed port, fail if that port is not available
  server: {
    port: 3000,
    strictPort: true, // 固定端口，占用时直接失败而不是漂移
    host: "127.0.0.1", // 明确绑定到本地回环
    hmr: false, // Disable HMR to avoid WebSocket permission issues
    watch: {
      // 3. tell Vite to ignore watching `src-tauri`
      ignored: ["**/src-tauri/**"],
    },
  },
  
  // 4. Build optimization - code splitting and asset optimization
  build: {
    rollupOptions: {
      output: {
        manualChunks: {
          // Core React ecosystem
          'vendor-react': ['react', 'react-dom', 'react-router-dom'],
          // UI components and icons
          'vendor-ui': ['lucide-react', 'framer-motion'],
          // Data fetching and state management
          'vendor-query': ['@tanstack/react-query', '@tanstack/react-virtual'],
          // State management
          'vendor-state': ['zustand', 'immer'],
          // Internationalization
          'vendor-i18n': ['react-i18next', 'i18next'],
          // Charts (if used)
          'vendor-charts': ['recharts'],
        },
      },
    },
    // Inline assets smaller than 4KB as base64
    assetsInlineLimit: 4096,
    // Enable CSS code splitting
    cssCodeSplit: true,
    // Chunk size warning limit (in KB)
    chunkSizeWarningLimit: 1000,
  },
}));

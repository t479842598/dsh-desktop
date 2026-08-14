import { defineConfig } from "vite";

// Tauri 需要固定的 dev server 端口，且禁用自动打开浏览器
export default defineConfig({
  clearScreen: false,
  server: {
    port: 1420,
    strictPort: true,
    watch: {
      ignored: ["**/src-tauri/**"],
    },
  },
  build: {
    target: "es2021",
    outDir: "dist",
  },
});

import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";
import path from "node:path";

// Tauri expects a fixed port and ignores HMR over the network host.
const host = process.env.TAURI_DEV_HOST;

// https://vitejs.dev/config/
export default defineConfig({
  plugins: [react()],
  resolve: {
    alias: { "@": path.resolve(__dirname, "./src") },
  },
  // Prevent Vite from obscuring Rust errors.
  clearScreen: false,
  server: {
    port: 5173,
    strictPort: true,
    host: host || false,
    hmr: host
      ? { protocol: "ws", host, port: 5174 }
      : undefined,
    watch: {
      // Don't watch the Rust backend; cargo handles that.
      ignored: ["**/src-tauri/**"],
    },
  },
  // Produce a bundle Tauri can serve from ../dist
  build: {
    target: "es2021",
    minify: "esbuild",
    sourcemap: false,
  },
});

import path from "path";
import { fileURLToPath } from "url";
import { defineConfig } from "vite";
import { svelte } from "@sveltejs/vite-plugin-svelte";

const __dirname = path.dirname(fileURLToPath(import.meta.url));

// https://vitejs.dev/config/
export default defineConfig({
  plugins: [svelte()],
  publicDir: "static",
  clearScreen: false,
  server: {
    port: 1420,
    strictPort: true,
    hmr: {
      overlay: false,
    },
    watch: {
      ignored: ["**/src-tauri/**"],
    },
  },
  resolve: {
    alias: {
      '$lib': path.resolve(__dirname, "src/lib"),
    },
  },
  build: {
    target: "es2021",
    sourcemap: false,
    rollupOptions: {
      output: {
        manualChunks: {
          charting: ["chart.js"],
          graph: ["cytoscape"],
          markdown: ["marked", "dompurify"],
        },
      },
    },
  },
});

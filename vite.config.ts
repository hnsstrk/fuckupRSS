import { defineConfig } from "vite";
import { svelte } from "@sveltejs/vite-plugin-svelte";

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
      '$lib': '/src/lib',
    },
  },
});

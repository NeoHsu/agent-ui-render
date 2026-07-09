import vue from "@vitejs/plugin-vue";
import { defineConfig } from "vite";

export default defineConfig({
  plugins: [vue()],
  build: {
    outDir: "../generated",
    emptyOutDir: false,
    minify: "esbuild",
    cssMinify: true,
    lib: {
      entry: "src/main.ts",
      name: "AgentUiRender",
      formats: ["iife"],
      fileName: () => "renderer.js",
    },
    rollupOptions: {
      output: {
        assetFileNames: (assetInfo: { names?: string[] }) =>
          assetInfo.names?.some((name: string) => name.endsWith(".css"))
            ? "renderer.css"
            : "[name][extname]",
      },
    },
  },
});

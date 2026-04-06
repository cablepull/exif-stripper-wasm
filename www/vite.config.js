import { defineConfig } from "vite";

// GitHub Pages project site: https://<user>.github.io/<repo>/
const pagesBase = "/exif-stripper-wasm/";

export default defineConfig({
  base: process.env.GITHUB_ACTIONS === "true" ? pagesBase : "/",
  server: {
    headers: {
      "Cross-Origin-Opener-Policy": "same-origin",
      "Cross-Origin-Embedder-Policy": "require-corp",
    },
  },
  optimizeDeps: {
    exclude: ["../pkg"],
  },
});

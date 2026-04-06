import path from "node:path";
import { fileURLToPath } from "node:url";
import { defineConfig } from "vite";

const wwwDir = path.dirname(fileURLToPath(import.meta.url));
const pkgDir = path.resolve(wwwDir, "../pkg");

// GitHub Pages project site: https://<user>.github.io/<repo>/
const pagesBase = "/exif-stripper-wasm/";

export default defineConfig({
  base: process.env.GITHUB_ACTIONS === "true" ? pagesBase : "/",
  resolve: {
    // Rollup cannot resolve `../../pkg/...` from `www/` during production build; alias to absolute path.
    alias: {
      "@wasm-pkg": pkgDir,
    },
  },
  server: {
    fs: {
      allow: [wwwDir, pkgDir],
    },
    headers: {
      "Cross-Origin-Opener-Policy": "same-origin",
      "Cross-Origin-Embedder-Policy": "require-corp",
    },
  },
  optimizeDeps: {
    exclude: ["@wasm-pkg"],
  },
  // wasm-bindgen glue uses async chunks; IIFE workers cannot code-split.
  worker: {
    format: "es",
  },
});

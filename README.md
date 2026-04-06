# EXIF Stripper (WASM)

Remove EXIF and other embedded metadata from JPEG and PNG images **in the browser**. Processing uses a Rust core compiled to WebAssembly; files are not uploaded to a server.

## Prerequisites

- [Rust](https://rustup.rs/) stable with the `wasm32-unknown-unknown` target (see [`rust-toolchain.toml`](rust-toolchain.toml))
- [wasm-pack](https://rustwasm.github.io/wasm-pack/installer/)
- Node.js 20+ (for the `www/` front-end)

## Build

From the repository root:

```bash
wasm-pack build --target web --release
cd www && npm ci && npm run build
```

- **`pkg/`** — JavaScript and WASM bindings produced by wasm-pack (load this path from `www/` during dev/build).
- **`www/dist/`** — Static site ready to deploy (includes hashed assets from Vite).

For a **local production build** that matches GitHub Pages asset paths (`/exif-stripper-wasm/…`), run:

```bash
cd www && GITHUB_ACTIONS=true npm run build
```

`GITHUB_ACTIONS` is set automatically in the GitHub workflow.

### Local development

```bash
wasm-pack build --target web
cd www && npm run dev
```

Open the URL Vite prints (typically `http://localhost:5173`).

## WASM binary integrity

After every `wasm-pack build`, you can fingerprint the compiled module:

```bash
shasum -a 256 pkg/exif_stripper_wasm_bg.wasm
```

Store the printed digest with your release notes or CI logs. Re-run the same command on another checkout or build artifact and **compare digests** to confirm the `.wasm` file matches.

## Deploy

- **Live site (GitHub Pages):** [https://cablepull.github.io/exif-stripper-wasm](https://cablepull.github.io/exif-stripper-wasm)
- **Source repository:** [https://github.com/cablepull/exif-stripper-wasm](https://github.com/cablepull/exif-stripper-wasm)
- **CI:** [`.github/workflows/deploy.yml`](.github/workflows/deploy.yml) builds on push to `main` and publishes `www/dist`.
- **Cloudflare Pages:** The [`www/public/_headers`](www/public/_headers) file is copied to the site root by Vite so responses can include CSP, COOP/COEP, and `X-Content-Type-Options`.

## Layout

| Path | Role |
|------|------|
| `src/` | Rust library and `wasm-bindgen` exports |
| `tests/` | Host-side integration tests (`cargo test --tests`) |
| `www/` | Vite app, Web Worker, UI |
| `intent.md`, `requirements.md`, `design.md`, `tasks.md` | Spec artifacts (see `CLAUDE.md` for the spec-check gate workflow) |

## Manual checks (not automated in CI)

- **Storage:** After processing, confirm DevTools → Application shows no image blobs in persistent storage (see `tasks.md` M5).
- **Network:** With the page loaded, process a file and confirm no new requests appear in the Network panel (beyond the initial page load).
- **Latency:** Use the per-file **processing time** shown in the UI to spot slow runs; the spec targets &lt; 100 ms for typical JPEGs under 10 MB on a warm tab.

## License

See the repository’s license file when one is added.

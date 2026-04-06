# Design

References: Feature F-1 (File Input), Feature F-2 (JPEG Metadata Removal),
Feature F-3 (PNG Metadata Removal), Feature F-4 (Metadata Diff Display),
Feature F-5 (Single File Download), Feature F-6 (Batch Download),
Feature F-7 (Client-Side Processing Guarantee), Feature F-8 (Performance)

## Requirement Traceability

| Rule | Criterion | Satisfied By |
|------|-----------|--------------|
| R-1  | Accepted file types are JPEG and PNG only | File Validator — MIME type and extension allowlist |
| R-2  | Per-file size limit enforced at 50MB | File Validator — size check before queue entry |
| R-3  | Batch input up to 50 files | Queue Manager — hard cap at 50 with inline warning |
| R-4  | All EXIF APP1 segments removed from JPEG | WASM Core → strip_jpeg() using img-parts |
| R-5  | Embedded thumbnails removed from JPEG | WASM Core → strip_jpeg() — APP1 segment excision removes embedded thumbnail |
| R-6  | Pixel data preserved without re-encoding | WASM Core — segment surgery only; no decode/re-encode path |
| R-7  | PNG metadata chunks removed | WASM Core → strip_png() — explicit chunk blocklist |
| R-8  | ICC profile and gamma chunks preserved | WASM Core → strip_png() — explicit chunk allowlist for iCCP and gAMA |
| R-9  | Metadata displayed in categories before download | Diff Renderer ← read_exif_tags() JSON output |
| R-10 | File size before and after shown | Result Row — Blob.size before vs after WASM call |
| R-11 | Single file downloadable after processing | Download Manager → URL.createObjectURL → anchor download |
| R-12 | Output filename prefixed with clean_ | Download Manager — filename construction rule |
| R-13 | Batch downloadable as ZIP | Batch Packager → ZIP library → Blob download |
| R-14 | No network requests after page load | CSP connect-src 'none' + Worker contains no fetch or XHR |
| R-15 | No residual file storage | Application contains no localStorage or IndexedDB writes |
| R-16 | Single file < 100ms processing | WASM Core — Rust binary performance; duration measured via performance.now() |
| R-17 | UI thread unblocked during processing | Web Worker — all WASM calls execute in worker thread |

## Components

```
┌────────────────────────────────────────────────────────────────────┐
│                         Browser Main Thread                        │
│                                                                    │
│  ┌─────────────┐   ┌────────────────┐   ┌────────────────────┐    │
│  │  Drop Zone  │   │ File Validator │   │  Queue Manager     │    │
│  │  + Picker   │ → │ (MIME + size)  │ → │  (max 50, state)   │    │
│  └─────────────┘   └────────────────┘   └─────────┬──────────┘    │
│                                                    │               │
│                                    postMessage(ArrayBuffer xfer)   │
│                                                    ▼               │
│  ┌─────────────────────────────────────────────────────────────┐   │
│  │                      Web Worker Thread                      │   │
│  │                                                             │   │
│  │   ┌─────────────────────────────────────────────────────┐  │   │
│  │   │                  WASM Core (Rust)                    │  │   │
│  │   │   strip_jpeg(data: &[u8]) → Result<Vec<u8>>          │  │   │
│  │   │   strip_png(data: &[u8])  → Result<Vec<u8>>          │  │   │
│  │   │   read_exif_tags(data: &[u8]) → Result<JsValue>      │  │   │
│  │   └─────────────────────────────────────────────────────┘  │   │
│  └─────────────────────────────────────────────────────────────┘   │
│                                                    │               │
│                                    postMessage(result | error)     │
│                                                    ▼               │
│  ┌──────────────────┐  ┌─────────────────┐  ┌─────────────────┐   │
│  │  Diff Renderer   │  │  Batch Packager │  │ Download Manager│   │
│  │  (tag categories)│  │  (ZIP library)  │  │ (ObjectURL)     │   │
│  └──────────────────┘  └─────────────────┘  └─────────────────┘   │
└────────────────────────────────────────────────────────────────────┘
```

## WASM Core

The WASM Core is a Rust library compiled to WebAssembly. It is the only component that directly
parses or modifies raw image bytes.

**Exposed API:**
- `strip_jpeg(data)` — iterates the JFIF container segment by segment using the img-parts crate,
  filters out all APP1 marker segments (which carry EXIF and embedded thumbnails), and returns the
  reassembled byte sequence. No pixel data is decoded or re-encoded.
- `strip_png(data)` — reads PNG chunks using the png crate, applies an explicit blocklist
  (`tEXt`, `zTXt`, `iTXt`, `eXIf`, `tIME`) and an explicit allowlist (`IHDR`, `IDAT`, `IEND`,
  `iCCP`, `gAMA`, `PLTE`, `tRNS`), and returns the reassembled chunk sequence.
- `read_exif_tags(data)` — parses EXIF tags using the kamadak-exif crate and returns a
  JSON-serialised array of `{id, name, category, value}` objects. Called before strip_jpeg so the
  diff view has the original metadata; does not modify the input.

**Error handling:** All three functions return `Result<_, JsValue>` rather than panicking. This
allows the Web Worker to catch per-file failures and report them as error messages without crashing
the worker thread.

**Key constraints:** The img-parts crate is chosen specifically because it performs segment-level
surgery without decoding image content. The png crate allowlist is explicit (not a blocklist of
unknowns) to avoid inadvertently preserving unknown future chunk types that carry metadata.

## Web Worker Bridge

Isolates all WASM execution from the main thread, satisfying R-17.

**Message protocol:**
- Main → Worker: `{ type: "process", id: string, buffer: ArrayBuffer, mime: string }` — the
  ArrayBuffer is transferred (not copied), so the main thread relinquishes ownership
- Worker → Main (success): `{ type: "result", id: string, buffer: ArrayBuffer, tags: ExifTag[],
  durationMs: number }`
- Worker → Main (failure): `{ type: "error", id: string, code: string, message: string }`

The Worker initialises the WASM module once on first message receipt and caches the instance.
Subsequent files do not re-pay the init cost.

## File Validator

Runs synchronously on the main thread before any file is handed to the Queue Manager. Enforces
R-1 and R-2.

**Validation sequence:**
1. Check `file.type` against the allowlist `["image/jpeg", "image/png"]`
2. Check `file.size <= 50 * 1024 * 1024`
3. On any failure, emit an inline error row immediately and do not queue the file

## Queue Manager

Maintains the ordered list of files awaiting processing and their per-file state
(`pending | processing | done | error`). Caps total files at 50 (R-3) and dispatches files to the
Worker one at a time in FIFO order. Updates the progress counter after each Worker result message.

## Diff Renderer

Receives the `tags` array from a Worker result message and renders a categorised summary in the
result row for that file. Category assignment rules:

| Category   | Tag name patterns                                              |
|------------|----------------------------------------------------------------|
| Location   | GPS* tags                                                     |
| Device     | Make, Model, SerialNumber, LensInfo, LensMake, LensModel      |
| Timestamps | DateTime*, GPS date/time fields                               |
| Software   | Software, ProcessingSoftware, HostComputer                    |
| Other      | All remaining tags                                            |

An empty `tags` array renders the message "No metadata found".

## Download Manager

Converts a processed ArrayBuffer to a Blob and constructs a download anchor. The output filename
is always `clean_` + original filename (R-12). The object URL is revoked immediately after the
anchor click is dispatched to release memory, since the Blob is no longer needed once the download
has been initiated.

## Batch Packager

Uses the fflate library to assemble a ZIP archive in memory from all completed file Blobs. The ZIP
is constructed only after all queued files have reached the `done` state; partial batch download is
not supported in v1. The resulting Blob is passed to the Download Manager for a single anchor-click
download.

## Deployment Architecture

Two tiers are defined. The WASM core and application logic are identical between tiers; only
infrastructure configuration differs.

**Demo tier (GitHub Pages):** No custom HTTP response headers. The privacy guarantee is behavioural
(the code does not send data) rather than header-enforced. SharedArrayBuffer is unavailable. Web
Worker communication must use transferable ArrayBuffers only.

**Production tier (Cloudflare Pages):** Custom headers are set via a `_headers` file committed to
the repository root. `connect-src 'none'` is enforced at the CDN edge (R-14). COOP and COEP headers
are set, making SharedArrayBuffer available for future performance work.

## Assumptions

| # | Assumption | Basis | Impact if wrong |
|---|-----------|-------|-----------------|
| A1 | Transferring an ArrayBuffer to the Web Worker is faster than structured clone for files in the 1–50MB range | Standard Web API behaviour for transferable objects; transfer is O(1), clone is O(n) | If transfer semantics change, the buffer must be cloned at higher memory cost |
| A2 | The WASM module can be initialised inside the Worker on Safari 15.2+ without COOP/COEP headers | Confirmed in Chrome and Firefox; Safari 15.2+ WASM-in-Worker support documented in release notes | If Safari requires COOP for WASM in Workers, the demo tier must use a same-thread fallback |
| A3 | Store-mode ZIP (no compression) is an acceptable fallback if DEFLATE compression misses the batch time target | The uncompressed ZIP path in fflate is ~10× faster than compressed; image files are not significantly compressible anyway | If even store-mode ZIP is too slow for the batch target, streaming chunked output to the Download Manager is required |

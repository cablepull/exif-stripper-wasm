# Tasks

## Milestone M1 — WASM Core and Basic UI

- [x] Initialise Cargo workspace with wasm32-unknown-unknown target and wasm-bindgen dependency (Rule: client-side processing guarantee)
- [x] Add img-parts, kamadak-exif, and png crates to Cargo.toml (Rule: JPEG metadata removal, Rule: PNG metadata removal)
- [x] Implement strip_jpeg() in lib.rs using img-parts: iterate JFIF segments, filter all APP1 markers, return reassembled bytes (Rule: all EXIF APP1 segments are removed from JPEG output)
- [x] Implement strip_png() in lib.rs using the png crate: read chunks, apply explicit blocklist for tEXt/zTXt/iTXt/eXIf/tIME, preserve iCCP and gAMA, return reassembled bytes (Rule: metadata-bearing ancillary chunks are removed from PNG output, Rule: ICC color profile and gamma chunks are preserved)
- [x] Implement read_exif_tags() in lib.rs using kamadak-exif: return JSON array of tag objects with id, name, category, and value fields (Rule: metadata found in a file is displayed in categories before download)
- [x] Return JsValue errors from all three WASM functions instead of panicking so per-file failures do not crash the Worker thread (Rule: UI thread is not blocked during processing)
- [x] Run wasm-pack build and confirm the pkg/ output directory is generated with correct JS bindings (Rule: no network requests are made after the page has loaded)
- [x] Create www/index.html with drop zone markup, file picker input, results container, and Download All button (Rule: accepted file types are JPEG and PNG only, Rule: batch input supports up to 50 files)
- [x] Implement MIME type and size validation in app.js before queuing any file (Rule: accepted file types are JPEG and PNG only, Rule: per-file size limit is enforced at 50MB)
- [x] Implement single file processing in app.js: FileReader to ArrayBuffer, WASM strip call, Blob construction, and download anchor (Rule: single cleaned file is downloadable immediately after processing)
- [x] Construct the output download filename as clean_ plus the original filename with original extension (Rule: output filename is prefixed with clean_ and retains the original extension)

## Milestone M2 — Metadata Diff View

- [x] Call read_exif_tags() on each file before calling the strip function and store the returned tag array (Rule: metadata found in a file is displayed in categories before download)
- [x] Implement Diff Renderer in app.js: map tags to Location, Device, Timestamps, Software, and Other categories and render collapsible sections in each result row (Rule: metadata found in a file is displayed in categories before download)
- [x] Display original and cleaned file sizes in each result row (Rule: file size before and after processing is displayed per file)
- [x] Flag embedded thumbnail presence in the diff view when tag data includes a thumbnail entry (Rule: embedded thumbnails are removed from JPEG output)
- [x] Render the message "No metadata found" for files with an empty tag array (Rule: metadata found in a file is displayed in categories before download)

## Milestone M3 — Batch Processing and Web Worker

- [x] Create worker.js: initialise WASM on first message, implement postMessage protocol for process/result/error messages, transfer ArrayBuffer ownership to Worker (Rule: UI thread is not blocked during processing)
- [x] Move all WASM calls from the main thread in app.js to worker.js (Rule: UI thread is not blocked during processing)
- [x] Implement Queue Manager in app.js: cap at 50 files, emit warning on excess, dispatch files to Worker in FIFO order, track per-file state as pending/processing/done/error (Rule: batch input supports up to 50 files per session)
- [x] Display a processing progress counter showing completed count out of total during batch jobs (Rule: a progress counter updates while batch processing is in progress)
- [x] Add the client-side ZIP library to the www/ directory and verify it loads correctly in the browser (Rule: all processed files can be downloaded as a single ZIP archive)
- [x] Implement Batch Packager: collect all completed ArrayBuffers, build a ZIP archive with clean_ prefixed filenames, trigger download via Download Manager (Rule: all processed files can be downloaded as a single ZIP archive)
- [x] Disable or hide the Download All as ZIP button until at least one file has reached the done state (Rule: the Download All as ZIP button is not available when no files have completed)

## Milestone M4 — Polish and Security

- [x] Add an ARIA live region to the results container so screen readers announce processing status updates (Rule: UI thread is not blocked during processing)
- [x] Verify the drop zone and all buttons are keyboard-navigable with visible focus indicators (Rule: UI thread is not blocked during processing)
- [x] Audit colour contrast for all text and interactive elements and confirm WCAG AA 4.5:1 ratio is met (Rule: UI thread is not blocked during processing)
- [x] Create a _headers file for Cloudflare Pages deployment with Content-Security-Policy including connect-src none, COOP, COEP, Referrer-Policy, and X-Content-Type-Options (Rule: no network requests are initiated after the page has loaded, Rule: CSP header blocks any attempted connection)
- [x] Add SRI hash attributes to any external script or style includes in index.html (Rule: no network requests are initiated after the page has loaded)
- [x] Create .github/workflows/deploy.yml for automated GitHub Pages deployment on push to main (Rule: client-side processing guarantee)
- [x] Create rust-toolchain.toml pinning the stable channel with the wasm32-unknown-unknown target (Rule: client-side processing guarantee)
- [x] Add a How it works modal to index.html with one paragraph explaining client-side processing and a link to the source repository (Rule: no network requests are initiated after the page has loaded)
- [x] Write README.md with build instructions, the WASM binary SHA-256 hash, and the verification command (Rule: no file data persists after the browser tab is closed)

## Milestone M5 — Hardening and Edge Cases

- [x] Test strip_jpeg() against a progressive JPEG and confirm the output is a valid progressive JPEG with no EXIF (Rule: pixel data is preserved without re-encoding)
- [x] Test strip_jpeg() against a JPEG with multi-segment APP1 markers and confirm all segments are removed (Rule: all EXIF APP1 segments are removed from JPEG output)
- [x] Test strip_jpeg() against a JPEG with no EXIF data and confirm the output is valid and unmodified in content (Rule: all EXIF APP1 segments are removed from JPEG output)
- [x] Test strip_png() against a PNG containing only IHDR, IDAT, and IEND chunks and confirm byte-for-byte identity with input (Rule: ICC color profile and gamma chunks are preserved)
- [x] Test strip_png() against a PNG with iCCP and gAMA chunks alongside metadata chunks and confirm both are retained in output (Rule: ICC color profile and gamma chunks are preserved)
- [x] Run exiftool on JPEG and PNG outputs and confirm zero tags are reported (Rule: all EXIF APP1 segments are removed from JPEG output, Rule: metadata-bearing ancillary chunks are removed from PNG output)
- [x] Run pngcheck -v on PNG outputs and confirm no unknown ancillary chunks are present (Rule: metadata-bearing ancillary chunks are removed from PNG output)
- [ ] Confirm localStorage and IndexedDB contain no image data after processing a file and navigating away (Rule: no file data persists after the browser tab is closed)
- [ ] Measure processing time for an 8-megapixel JPEG and confirm the result is under 100 milliseconds (Rule: single file processing completes within 100ms for files under 10MB)
- [ ] Confirm the browser network panel shows zero new requests after page load when a file is processed (Rule: no network requests are initiated after the page has loaded)

## Assumptions

| # | Assumption | Basis | Impact if wrong |
|---|-----------|-------|-----------------|
| A1 | Milestones M1–M5 can be completed in order without unblocking dependencies between them | Each milestone produces a working increment; M2 depends on the WASM tag-reading function from M1, M3 depends on the Worker from M1 | If M1 deliverables are incomplete, M2 and M3 tasks cannot begin |
| A2 | The Rust toolchain and wasm-pack can be installed without system administrator privileges on the development machine | Standard cargo install workflow; not verified on restricted corporate environments | If elevated permissions are required, a pre-built pkg/ directory committed to the repository would be needed |
| A3 | Rust integration tests in `tests/strip_tests.rs` cover M5 core cases; exiftool and pngcheck run when those tools are installed | Automated tests assert structure; optional CLI tools skip when absent | If CI must prove exiftool/pngcheck without installing them, add a job that installs those tools or use fixture-only assertions |

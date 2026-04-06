# Intent

## Problem

The problem is that photographs silently carry private data and users have no trustworthy way to
remove it. Every image captured by a modern phone or camera embeds GPS coordinates, device serial
numbers, camera make and model, software version strings, and precise timestamps directly into the
file. This data travels invisibly with every image a user posts to a marketplace listing, sends by
email, or shares online. The vast majority of users are unaware the problem exists, and fewer still
know how to verify whether their image has been cleaned.

The problem is compounded by inadequate existing options. Cloud-based services require a user to
upload a private photo to a third-party server in order to remove its private metadata — a
contradiction that makes the privacy problem worse. JavaScript parsers running in the browser are
unreliable: they miss embedded thumbnails, malformed-but-valid segment structures, and progressive
container formats, so users receive a false assurance of cleanliness. Neither category of existing
tool is trustworthy for the use cases where this matters most.

## Why this must be solved in the browser

Because the only trustworthy metadata removal is one the user can verify themselves. A server-side
service introduces an unverifiable trust relationship: the user must believe the provider does not
log, retain, or process the image, and there is no technical means to confirm this. A compiled
binary running entirely in the browser can be audited against its source, its output verified with
standard tools, and its network behaviour confirmed in any developer tools panel. There is no
equivalent verification path for a cloud service.

Compiled code running in the browser enables correct binary container parsing — the same quality
achievable in a native binary — without any server infrastructure, without requiring the user to
install anything, and without any file leaving the device. Therefore the solution is a static web
application with a compiled processing core, not a service.

## Intent

Build a browser-native application that removes EXIF and metadata from JPEG and PNG images entirely
on the user's device, with no network requests after the page loads. The application must produce
verifiably clean output — confirmed by standard inspection tools, not just asserted in copy — and
must be deployable as a static file directory so that organisations with air-gapped requirements can
self-host it without a server runtime.

The application must also show users exactly what metadata was present before removal, so the
guarantee is observable rather than implied. This transparency is what separates a trustworthy tool
from a black box.

## What this is not

This is not a cloud service. No file data is transmitted to any server.
This is not a metadata editor. The application removes metadata; it does not allow selective
retention or field-level editing.
This is not a mobile native application. The delivery mechanism is a standard browser page.
This is not a file format converter. Pixel data is never re-encoded; only container segments are
modified.
This is not an installer. Users open a URL or a local file; nothing is installed on their machine.

## Constraints

- Zero network requests after initial page load — enforced by Content Security Policy
- No server runtime — the application must be deployable as a static directory of files
- No residual storage of user files — no writes to the browser's persistent storage APIs
- Must function correctly when served from a local filesystem (file:// protocol)
- Output must be reproducibly verifiable: running exiftool on the output must report zero tags
- Must support batch processing without blocking the browser UI thread
- Source code must be open and the compiled binary must be buildable from source so users can
  verify the published hash independently

## Assumptions

| # | Assumption | Basis | Impact if wrong |
|---|-----------|-------|-----------------|
| A1 | The four target browsers (Chrome 90+, Firefox 89+, Safari 15.2+, Edge 90+) support WebAssembly and Web Workers without flags | MDN compatibility tables as of 2026; all four browsers have shipped these APIs in stable releases | A JavaScript fallback parser would be required for any browser lacking WASM or Worker support |
| A2 | Users can load and run the application from a local filesystem for air-gapped use | Inferred from the air-gapped self-hosting requirement; not confirmed by user research | If file:// is blocked by policy in target environments, a minimal local HTTP server wrapper would be required |
| A3 | JPEG and PNG cover the majority of the privacy-sensitive use cases for the primary personas | Derived from the primary user groups (journalists, online sellers, security researchers) who primarily share phone-captured photos in JPEG or PNG format | If a significant use case requires HEIC or RAW format support, the processing core would need additional format libraries |
| A4 | Container-level segment surgery preserves image quality and is accepted by all major image viewers | Confirmed by img-parts and png crate documentation; requires validation against edge cases during hardening | If a viewer rejects a surgically-cleaned container, a fallback re-encode path must be added |
| A5 | Client-side ZIP generation is fast enough to package batches of up to 50 × 50MB files within the 3-second batch target | Based on published benchmark data for the planned ZIP library; not yet measured in-browser on target hardware | If packaging is too slow, streaming ZIP generation or store-mode (no compression) ZIP would be required |

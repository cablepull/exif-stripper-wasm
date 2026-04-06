/// Integration tests for the EXIF-stripper WASM core.
///
/// These tests run on the native host target (not wasm32) so they can be
/// executed with `cargo test` without any browser or WASM runtime.
use exif_stripper_wasm::core::{strip_jpeg_inner as strip_jpeg, strip_png_inner as strip_png, read_exif_tags_inner as read_exif_tags};

// ── Minimal valid JPEG fixture ─────────────────────────────────────────────────
//
// SOI  FF D8
// APP0 FF E0  00 10  (JFIF marker, 16-byte payload)
//            4A 46 49 46 00  (JFIF\0)
//            01 01 00 00 01 00 01 00 00   (version, density, thumbnail)
// APP1 FF E1  00 08  (EXIF-like APP1, 8-byte payload — all zeros)
//            45 78 69 66 00 00  (Exif\0\0)
// SOF0 FF C0 … skipped for minimal fixture
// EOI  FF D9
//
// We build a minimal JPEG with one APP1 segment to verify strip_jpeg removes it.
fn minimal_jpeg_with_app1() -> Vec<u8> {
    let mut v: Vec<u8> = Vec::new();
    // SOI
    v.extend_from_slice(&[0xFF, 0xD8]);
    // APP0 (JFIF) — marker + length (16) + 14 bytes payload
    v.extend_from_slice(&[0xFF, 0xE0, 0x00, 0x10]);
    v.extend_from_slice(b"JFIF\x00\x01\x01\x00\x00\x01\x00\x01\x00\x00");
    // APP1 (EXIF stub) — marker + length (10) + "Exif\0\0" + 4 zero bytes
    v.extend_from_slice(&[0xFF, 0xE1, 0x00, 0x0A]);
    v.extend_from_slice(b"Exif\x00\x00\x00\x00");
    // EOI
    v.extend_from_slice(&[0xFF, 0xD9]);
    v
}

/// Minimal JPEG with no EXIF / no APP1 segment.
fn minimal_jpeg_no_exif() -> Vec<u8> {
    let mut v: Vec<u8> = Vec::new();
    v.extend_from_slice(&[0xFF, 0xD8]);
    v.extend_from_slice(&[0xFF, 0xE0, 0x00, 0x10]);
    v.extend_from_slice(b"JFIF\x00\x01\x01\x00\x00\x01\x00\x01\x00\x00");
    v.extend_from_slice(&[0xFF, 0xD9]);
    v
}

// ── Minimal valid PNG fixture ─────────────────────────────────────────────────
//
// PNG signature + IHDR (minimal 1×1 8-bit greyscale) + IDAT + IEND.
// We optionally inject a tEXt chunk to test removal.
fn minimal_png_with_text_chunk() -> Vec<u8> {
    // PNG signature
    let sig: &[u8] = &[0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A];

    // IHDR: length=13, type=IHDR, 1×1 8-bit greyscale non-interlaced + CRC
    let ihdr_data: &[u8] = &[
        0x00, 0x00, 0x00, 0x01, // width = 1
        0x00, 0x00, 0x00, 0x01, // height = 1
        0x08,                   // bit depth = 8
        0x00,                   // colour type = greyscale
        0x00,                   // compression = deflate
        0x00,                   // filter = adaptive
        0x00,                   // interlace = none
    ];
    let ihdr_chunk = png_chunk(b"IHDR", ihdr_data);

    // tEXt chunk: keyword "Comment\0" + value "test"
    let text_payload = b"Comment\x00test";
    let text_chunk = png_chunk(b"tEXt", text_payload);

    // IDAT: minimal deflate-compressed 1×1 greyscale pixel (value 0)
    // Compressed form: deflate of [0x00, 0x00] (filter byte + pixel)
    let idat_payload: &[u8] = &[
        0x08, 0xD7, 0x63, 0x60, 0x60, 0x00, 0x00, 0x00, 0x02, 0x00, 0x01,
    ];
    let idat_chunk = png_chunk(b"IDAT", idat_payload);

    // IEND: empty
    let iend_chunk = png_chunk(b"IEND", &[]);

    let mut out = Vec::new();
    out.extend_from_slice(sig);
    out.extend_from_slice(&ihdr_chunk);
    out.extend_from_slice(&text_chunk);
    out.extend_from_slice(&idat_chunk);
    out.extend_from_slice(&iend_chunk);
    out
}

/// Build a raw PNG chunk: 4-byte length + 4-byte type + data + 4-byte CRC.
fn png_chunk(kind: &[u8; 4], data: &[u8]) -> Vec<u8> {
    let len = data.len() as u32;
    let mut chunk = Vec::new();
    chunk.extend_from_slice(&len.to_be_bytes());
    chunk.extend_from_slice(kind);
    chunk.extend_from_slice(data);
    // CRC over type + data
    let crc = crc32_ieee(kind, data);
    chunk.extend_from_slice(&crc.to_be_bytes());
    chunk
}

/// CRC-32/ISO-HDLC (same polynomial as PNG CRC).
fn crc32_ieee(kind: &[u8; 4], data: &[u8]) -> u32 {
    // Pre-computed CRC table (standard IEEE 802.3 polynomial 0xEDB88320).
    fn make_table() -> [u32; 256] {
        let mut table = [0u32; 256];
        for n in 0..256u32 {
            let mut c = n;
            for _ in 0..8 {
                if c & 1 != 0 {
                    c = 0xEDB88320 ^ (c >> 1);
                } else {
                    c >>= 1;
                }
            }
            table[n as usize] = c;
        }
        table
    }
    let table = make_table();
    let mut crc: u32 = 0xFFFF_FFFF;
    for &b in kind.iter().chain(data.iter()) {
        let idx = ((crc ^ b as u32) & 0xFF) as usize;
        crc = table[idx] ^ (crc >> 8);
    }
    crc ^ 0xFFFF_FFFF
}

// ── JPEG tests ─────────────────────────────────────────────────────────────────

#[test]
fn strip_jpeg_removes_app1() {
    let input = minimal_jpeg_with_app1();
    let output = strip_jpeg(&input).expect("strip_jpeg should succeed");

    // Output must be a JPEG — starts with SOI marker FF D8.
    assert!(output.starts_with(&[0xFF, 0xD8]), "output must start with JPEG SOI");

    // APP1 marker (FF E1) must not appear in the output.
    let no_app1 = !output.windows(2).any(|w| w == [0xFF, 0xE1]);
    assert!(no_app1, "output must not contain any APP1 (FF E1) segment");
}

#[test]
fn strip_jpeg_no_exif_is_valid() {
    let input = minimal_jpeg_no_exif();
    let output = strip_jpeg(&input).expect("strip_jpeg should succeed on EXIF-free JPEG");
    assert!(output.starts_with(&[0xFF, 0xD8]), "output must start with JPEG SOI");
}

#[test]
fn strip_jpeg_rejects_garbage() {
    let garbage = b"not a jpeg at all";
    let result = strip_jpeg(garbage);
    assert!(result.is_err(), "strip_jpeg must return Err for invalid input");
}

// ── PNG tests ──────────────────────────────────────────────────────────────────

#[test]
fn strip_png_removes_text_chunk() {
    let input = minimal_png_with_text_chunk();
    let output = strip_png(&input).expect("strip_png should succeed");

    // Output must begin with PNG signature.
    assert!(output.starts_with(&[0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A]),
        "output must begin with PNG signature");

    // tEXt chunk type bytes must not appear in the output.
    let has_text = output.windows(4).any(|w| w == *b"tEXt");
    assert!(!has_text, "output must not contain tEXt chunk");
}

#[test]
fn strip_png_preserves_ihdr_idat_iend() {
    let input = minimal_png_with_text_chunk();
    let output = strip_png(&input).expect("strip_png should succeed");

    let has_ihdr = output.windows(4).any(|w| w == *b"IHDR");
    let has_idat = output.windows(4).any(|w| w == *b"IDAT");
    let has_iend = output.windows(4).any(|w| w == *b"IEND");
    assert!(has_ihdr, "IHDR must be preserved");
    assert!(has_idat, "IDAT must be preserved");
    assert!(has_iend, "IEND must be preserved");
}

#[test]
fn strip_png_rejects_garbage() {
    let result = strip_png(b"not a png");
    assert!(result.is_err(), "strip_png must return Err for invalid input");
}

// ── EXIF tag reading tests ────────────────────────────────────────────────────

#[test]
fn read_exif_tags_returns_empty_for_no_exif() {
    let input = minimal_jpeg_no_exif();
    let json = read_exif_tags(&input).expect("read_exif_tags should not error on EXIF-free JPEG");
    assert_eq!(json.trim(), "[]", "should return empty array when no EXIF is present");
}

#[test]
fn read_exif_tags_returns_valid_json_for_no_exif() {
    // A JPEG with no EXIF data at all should return an empty JSON array.
    let input = minimal_jpeg_no_exif();
    let json = read_exif_tags(&input).expect("read_exif_tags should not error on EXIF-free JPEG");
    assert!(json.starts_with('['), "result must be a JSON array");
    assert!(json.ends_with(']'), "result must be a JSON array");
}

#[test]
fn read_exif_tags_errors_on_truncated_exif() {
    // A JPEG with a malformed (truncated) APP1 payload should return an Err.
    let input = minimal_jpeg_with_app1();
    // The stub APP1 has only 8 payload bytes — not a valid TIFF/EXIF block.
    // kamadak-exif will return a parse error, which we propagate.
    let result = read_exif_tags(&input);
    assert!(result.is_err(), "truncated EXIF payload should produce an error");
}

// ══════════════════════════════════════════════════════════════════════════════
// M5 — Hardening and edge-case tests
// ══════════════════════════════════════════════════════════════════════════════

// ── Additional JPEG fixtures ──────────────────────────────────────────────────

/// JPEG with two APP1 segments (multi-segment EXIF scenario).
fn jpeg_with_multiple_app1() -> Vec<u8> {
    let mut v: Vec<u8> = Vec::new();
    v.extend_from_slice(&[0xFF, 0xD8]);
    // APP0
    v.extend_from_slice(&[0xFF, 0xE0, 0x00, 0x10]);
    v.extend_from_slice(b"JFIF\x00\x01\x01\x00\x00\x01\x00\x01\x00\x00");
    // First APP1
    v.extend_from_slice(&[0xFF, 0xE1, 0x00, 0x0A]);
    v.extend_from_slice(b"Exif\x00\x00\x00\x00");
    // Second APP1 (XMP-style — starts with "http:")
    let xmp = b"http://ns.adobe.com/xap/1.0/\x00<x:xmpmeta/>";
    let xmp_len = (xmp.len() as u16 + 2).to_be_bytes();
    v.extend_from_slice(&[0xFF, 0xE1]);
    v.extend_from_slice(&xmp_len);
    v.extend_from_slice(xmp);
    // EOI
    v.extend_from_slice(&[0xFF, 0xD9]);
    v
}

/// A minimal progressive JPEG.
///
/// Progressive JPEGs use SOF2 (FF C2) instead of SOF0 (FF C0).
/// We embed a tiny 1×1 progressive JPEG in raw bytes — just enough structure
/// for img-parts to parse it as a valid JPEG with a progressive marker.
fn minimal_progressive_jpeg() -> Vec<u8> {
    let mut v: Vec<u8> = Vec::new();
    v.extend_from_slice(&[0xFF, 0xD8]); // SOI
    // APP0
    v.extend_from_slice(&[0xFF, 0xE0, 0x00, 0x10]);
    v.extend_from_slice(b"JFIF\x00\x01\x01\x00\x00\x01\x00\x01\x00\x00");
    // APP1 (EXIF stub to be stripped)
    v.extend_from_slice(&[0xFF, 0xE1, 0x00, 0x0A]);
    v.extend_from_slice(b"Exif\x00\x00\x00\x00");
    // SOF2 — marks this as a progressive JPEG (length 11 = 2+9)
    // precision=8, height=1, width=1, components=1
    v.extend_from_slice(&[0xFF, 0xC2, 0x00, 0x0B]);
    v.extend_from_slice(&[0x08, 0x00, 0x01, 0x00, 0x01, 0x01, 0x01, 0x11, 0x00]);
    // EOI
    v.extend_from_slice(&[0xFF, 0xD9]);
    v
}

// ── Additional PNG fixtures ───────────────────────────────────────────────────

/// PNG with only IHDR + IDAT + IEND (no ancillary chunks).
fn minimal_png_no_ancillary() -> Vec<u8> {
    let sig: &[u8] = &[0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A];
    let ihdr_data: &[u8] = &[
        0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01,
        0x08, 0x00, 0x00, 0x00, 0x00,
    ];
    let idat_payload: &[u8] = &[
        0x08, 0xD7, 0x63, 0x60, 0x60, 0x00, 0x00, 0x00, 0x02, 0x00, 0x01,
    ];
    let mut out = Vec::new();
    out.extend_from_slice(sig);
    out.extend_from_slice(&png_chunk(b"IHDR", ihdr_data));
    out.extend_from_slice(&png_chunk(b"IDAT", idat_payload));
    out.extend_from_slice(&png_chunk(b"IEND", &[]));
    out
}

/// PNG with iCCP + gAMA + tEXt chunks (iCCP and gAMA must survive stripping).
fn png_with_iccp_gama_text() -> Vec<u8> {
    let sig: &[u8] = &[0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A];
    let ihdr_data: &[u8] = &[
        0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01,
        0x08, 0x02, 0x00, 0x00, 0x00,
    ];
    // gAMA: gamma = 45455 (≈ 1/2.2), stored as u32 BE × 100000
    let gama_data = 45455u32.to_be_bytes();
    // iCCP: "sRGB\0\0" + 1 byte compression method + empty compressed profile
    // (real profiles are large; we use a stub that img-parts will round-trip)
    let iccp_data = b"sRGB\x00\x00";
    // tEXt to be removed
    let text_data = b"Author\x00test";
    let idat_payload: &[u8] = &[
        0x08, 0xD7, 0x63, 0xF8, 0xFF, 0xFF, 0x3F, 0x00, 0x05, 0xFE, 0x02, 0xFE,
        0xA7, 0x35, 0x81, 0x84,
    ];
    let mut out = Vec::new();
    out.extend_from_slice(sig);
    out.extend_from_slice(&png_chunk(b"IHDR", ihdr_data));
    out.extend_from_slice(&png_chunk(b"gAMA", &gama_data));
    out.extend_from_slice(&png_chunk(b"iCCP", iccp_data));
    out.extend_from_slice(&png_chunk(b"tEXt", text_data));
    out.extend_from_slice(&png_chunk(b"IDAT", idat_payload));
    out.extend_from_slice(&png_chunk(b"IEND", &[]));
    out
}

// ── M5 JPEG tests ─────────────────────────────────────────────────────────────

#[test]
fn strip_jpeg_removes_all_multiple_app1_segments() {
    // Rule: all EXIF APP1 segments are removed from JPEG output
    let input = jpeg_with_multiple_app1();
    // Confirm input has two APP1 markers before stripping
    let app1_count_in = input.windows(2).filter(|w| *w == [0xFF, 0xE1]).count();
    assert_eq!(app1_count_in, 2, "fixture must contain two APP1 segments");

    let output = strip_jpeg(&input).expect("strip_jpeg should succeed");
    assert!(output.starts_with(&[0xFF, 0xD8]), "output must start with JPEG SOI");

    let app1_count_out = output.windows(2).filter(|w| *w == [0xFF, 0xE1]).count();
    assert_eq!(app1_count_out, 0, "all APP1 segments must be removed");
}

#[test]
fn strip_jpeg_preserves_progressive_marker() {
    // Rule: pixel data is preserved without re-encoding
    let input = minimal_progressive_jpeg();
    let output = strip_jpeg(&input).expect("strip_jpeg should succeed on progressive JPEG");

    assert!(output.starts_with(&[0xFF, 0xD8]), "output must start with JPEG SOI");

    // SOF2 marker (FF C2) must still be present — proves progressive structure survived
    let has_sof2 = output.windows(2).any(|w| w == [0xFF, 0xC2]);
    assert!(has_sof2, "progressive SOF2 marker must be preserved");

    // APP1 must be gone
    let has_app1 = output.windows(2).any(|w| w == [0xFF, 0xE1]);
    assert!(!has_app1, "APP1 must be removed from progressive JPEG");
}

#[test]
fn strip_jpeg_no_exif_content_unchanged() {
    // Rule: all EXIF APP1 segments are removed from JPEG output
    // A JPEG with no EXIF — stripping should not alter any segment payload.
    let input = minimal_jpeg_no_exif();
    let output = strip_jpeg(&input).expect("strip_jpeg should succeed");

    // The APP0 (JFIF) payload in the input must appear verbatim in the output.
    // Fixture layout: SOI(2) + APP0-marker(2) + length(2) + 14-byte JFIF payload + EOI(2)
    // Payload bytes in input start at offset 6 and are 14 bytes long.
    let app0_payload_in = &input[6..20]; // 14-byte JFIF payload
    let app0_start = output.windows(2).position(|w| w == [0xFF, 0xE0])
        .expect("APP0 must be present in output");
    // In output, APP0-marker(2) + length(2) = 4 bytes before the payload.
    let app0_payload_out = &output[app0_start + 4 .. app0_start + 18];
    assert_eq!(app0_payload_in, app0_payload_out, "APP0 payload must be preserved unchanged");
}

// ── M5 PNG tests ──────────────────────────────────────────────────────────────

#[test]
fn strip_png_no_ancillary_is_byte_identical() {
    // Rule: ICC color profile and gamma chunks are preserved
    // A PNG with no ancillary chunks should survive strip_png byte-for-byte.
    let input = minimal_png_no_ancillary();
    let output = strip_png(&input).expect("strip_png should succeed");
    assert_eq!(input, output, "PNG with no ancillary chunks must be byte-for-byte identical after stripping");
}

#[test]
fn strip_png_preserves_iccp_and_gama() {
    // Rule: ICC color profile and gamma chunks are preserved
    let input = png_with_iccp_gama_text();
    let output = strip_png(&input).expect("strip_png should succeed");

    let has_gama = output.windows(4).any(|w| w == *b"gAMA");
    let has_iccp = output.windows(4).any(|w| w == *b"iCCP");
    let has_text = output.windows(4).any(|w| w == *b"tEXt");

    assert!(has_gama, "gAMA chunk must be preserved");
    assert!(has_iccp, "iCCP chunk must be preserved");
    assert!(!has_text, "tEXt chunk must be removed");
}

// ── M5 exiftool / pngcheck integration ───────────────────────────────────────

/// Run exiftool on bytes written to a temp file.
/// Returns the count of EXIF and XMP tags only (not file-system metadata or
/// JFIF/PNG structural fields that are not removable metadata).
/// Returns `usize::MAX` if exiftool is not available (caller should skip).
fn exiftool_metadata_tag_count(data: &[u8], ext: &str) -> usize {
    use std::io::Write;
    let path = std::env::temp_dir().join(format!("exif_test_{}.{}", std::process::id(), ext));
    std::fs::File::create(&path).unwrap().write_all(data).unwrap();
    let out = std::process::Command::new("exiftool")
        .args(["-EXIF:all", "-XMP:all", "-IPTC:all", "-s", "-s", "-s", path.to_str().unwrap()])
        .output();
    let _ = std::fs::remove_file(&path);
    match out {
        Ok(o) => {
            let text = String::from_utf8_lossy(&o.stdout);
            text.lines().filter(|l| !l.trim().is_empty() && !l.starts_with("Warning")).count()
        }
        Err(_) => usize::MAX,
    }
}

#[test]
fn exiftool_reports_zero_exif_tags_on_stripped_jpeg() {
    // Rule: all EXIF APP1 segments are removed from JPEG output
    let input = minimal_jpeg_with_app1();
    let output = strip_jpeg(&input).expect("strip_jpeg should succeed");

    let count = exiftool_metadata_tag_count(&output, "jpg");
    if count == usize::MAX { return; } // exiftool absent — skip
    assert_eq!(count, 0, "exiftool must report zero EXIF/XMP/IPTC tags on stripped JPEG; got {count}");
}

#[test]
fn exiftool_reports_zero_exif_tags_on_stripped_png() {
    // Rule: metadata-bearing ancillary chunks are removed from PNG output
    let input = minimal_png_with_text_chunk();
    let output = strip_png(&input).expect("strip_png should succeed");

    let count = exiftool_metadata_tag_count(&output, "png");
    if count == usize::MAX { return; }
    assert_eq!(count, 0, "exiftool must report zero EXIF/XMP/IPTC tags on stripped PNG; got {count}");
}

#[test]
fn pngcheck_validates_stripped_png() {
    // Rule: metadata-bearing ancillary chunks are removed from PNG output
    use std::io::Write;
    let input = minimal_png_with_text_chunk();
    let output = strip_png(&input).expect("strip_png should succeed");

    let path = std::env::temp_dir().join(format!("pngcheck_{}.png", std::process::id()));
    std::fs::File::create(&path).unwrap().write_all(&output).unwrap();
    let result = std::process::Command::new("pngcheck")
        .args(["-v", path.to_str().unwrap()])
        .output();
    let _ = std::fs::remove_file(&path);

    match result {
        Ok(o) => {
            let stdout = String::from_utf8_lossy(&o.stdout);
            // pngcheck -v must not report any unknown ancillary chunks
            let has_unknown = stdout.lines().any(|l| l.contains("unknown") || l.contains("UNKNOWN"));
            assert!(!has_unknown, "pngcheck must not report unknown chunks: {stdout}");
        }
        Err(_) => {} // pngcheck absent — skip
    }
}

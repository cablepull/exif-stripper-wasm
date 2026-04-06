use img_parts::{ImageICC, ImageEXIF};
use img_parts::jpeg::{Jpeg, markers};
use img_parts::png::Png;
use serde::Serialize;

// ── WASM bindings (only compiled for wasm32) ──────────────────────────────────
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub fn strip_jpeg(data: &[u8]) -> Result<Vec<u8>, JsValue> {
    core::strip_jpeg_inner(data).map_err(|e| JsValue::from_str(&e))
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub fn strip_png(data: &[u8]) -> Result<Vec<u8>, JsValue> {
    core::strip_png_inner(data).map_err(|e| JsValue::from_str(&e))
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub fn read_exif_tags(data: &[u8]) -> Result<String, JsValue> {
    core::read_exif_tags_inner(data).map_err(|e| JsValue::from_str(&e))
}

// ── Pure Rust core (callable from tests on any target) ────────────────────────
pub mod core {
    use super::*;

    /// Blocked ancillary PNG chunk types that carry metadata.
    const PNG_BLOCKLIST: &[[u8; 4]] = &[
        *b"tEXt", *b"zTXt", *b"iTXt",
        *b"eXIf",
        *b"tIME",
        *b"bKGD",
        *b"hIST",
        *b"sPLT",
    ];

    /// Remove all APP1 (EXIF/XMP) and APP13 (IPTC) segments from a JPEG.
    pub fn strip_jpeg_inner(data: &[u8]) -> Result<Vec<u8>, String> {
        let mut jpeg = Jpeg::from_bytes(data.to_vec().into())
            .map_err(|e| format!("JPEG parse error: {e}"))?;

        jpeg.segments_mut().retain(|seg| {
            !matches!(seg.marker(), markers::APP1 | markers::APP13 | markers::COM)
        });
        jpeg.set_icc_profile(None);
        jpeg.set_exif(None);

        Ok(jpeg.encoder().bytes().to_vec())
    }

    /// Remove metadata-bearing ancillary chunks from a PNG while preserving
    /// iCCP and gAMA.
    pub fn strip_png_inner(data: &[u8]) -> Result<Vec<u8>, String> {
        let mut png = Png::from_bytes(data.to_vec().into())
            .map_err(|e| format!("PNG parse error: {e}"))?;

        png.chunks_mut().retain(|chunk| {
            !PNG_BLOCKLIST.contains(&chunk.kind())
        });

        Ok(png.encoder().bytes().to_vec())
    }

    /// Parse EXIF tags and return a JSON array of tag objects.
    pub fn read_exif_tags_inner(data: &[u8]) -> Result<String, String> {
        let mut cursor = std::io::Cursor::new(data);
        let exif_reader = exif::Reader::new();

        let exif = match exif_reader.read_from_container(&mut cursor) {
            Ok(e) => e,
            Err(exif::Error::NotFound(_)) => return Ok("[]".to_string()),
            Err(e) => return Err(format!("EXIF read error: {e}")),
        };

        let tags: Vec<TagEntry> = exif
            .fields()
            .map(|f| TagEntry {
                id: f.tag.number(),
                name: f.tag.to_string(),
                category: categorise(&f.tag).to_string(),
                value: f.display_value().with_unit(&exif).to_string(),
            })
            .collect();

        serde_json::to_string(&tags).map_err(|e| format!("JSON serialise error: {e}"))
    }
}

// ── Types & helpers ───────────────────────────────────────────────────────────

#[derive(Serialize)]
struct TagEntry {
    id: u16,
    name: String,
    category: String,
    value: String,
}

fn categorise(tag: &exif::Tag) -> &'static str {
    use exif::Tag;
    match *tag {
        Tag::GPSLatitude
        | Tag::GPSLongitude
        | Tag::GPSAltitude
        | Tag::GPSLatitudeRef
        | Tag::GPSLongitudeRef
        | Tag::GPSAltitudeRef
        | Tag::GPSTimeStamp
        | Tag::GPSDateStamp
        | Tag::GPSImgDirection
        | Tag::GPSImgDirectionRef
        | Tag::GPSDestLatitude
        | Tag::GPSDestLongitude
        | Tag::GPSSpeed
        | Tag::GPSSpeedRef => "Location",

        Tag::Make | Tag::Model => "Device",

        Tag::DateTime | Tag::DateTimeOriginal | Tag::DateTimeDigitized => "Timestamps",

        Tag::Software => "Software",

        _ => "Other",
    }
}

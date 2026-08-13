//! Icon configuration: read and update the <mod-root>/assets/icon/
//! directory convention used by thrash-machine's build scripts.
//!
//! The frontend never sends file paths (the webview hands over a File
//! object, not a filesystem path), so it reads the picked image into a
//! base64 data URL and we decode, resize to the slot's target size, and
//! write it back to the convention location. Thumbnails are emitted the
//! same way (data URLs) so the UI can render them without filesystem access.

use crate::commands::AppState;
use base64::engine::general_purpose::STANDARD;
use base64::Engine as _;
use image::imageops::FilterType;
use image::{DynamicImage, GenericImageView};
use serde_json::{json, Value};
use std::path::{Path, PathBuf};
use tauri::State;

/// A single icon slot: a convention path plus the size the build wants it
/// at. `size: None` (the game window icon) is saved as-is.
struct SlotSpec {
    key: &'static str,
    group: &'static str,
    label: &'static str,
    rel: &'static str,
    size: Option<(u32, u32)>,
}

// Mirrors assets/icon/{window_icon.png, win/, android/} in build_standalone.sh
// and build_android.sh (win sizes feed the .exe icon; android densities use
// the standard Android dp sizes: ldpi 36, mdpi 48, hdpi 72, xhdpi 96,
// xxhdpi 144, xxxhdpi 192).
const SLOTS: &[SlotSpec] = &[
    SlotSpec { key: "window", group: "window", label: "window", rel: "window_icon.png", size: None },
    SlotSpec { key: "win-16", group: "win", label: "16×16", rel: "win/16x16.png", size: Some((16, 16)) },
    SlotSpec { key: "win-32", group: "win", label: "32×32", rel: "win/32x32.png", size: Some((32, 32)) },
    SlotSpec { key: "win-48", group: "win", label: "48×48", rel: "win/48x48.png", size: Some((48, 48)) },
    SlotSpec { key: "win-64", group: "win", label: "64×64", rel: "win/64x64.png", size: Some((64, 64)) },
    SlotSpec { key: "win-128", group: "win", label: "128×128", rel: "win/128x128.png", size: Some((128, 128)) },
    SlotSpec { key: "win-256", group: "win", label: "256×256", rel: "win/256x256.png", size: Some((256, 256)) },
    SlotSpec { key: "android-ldpi", group: "android", label: "ldpi", rel: "android/ldpi.png", size: Some((36, 36)) },
    SlotSpec { key: "android-mdpi", group: "android", label: "mdpi", rel: "android/mdpi.png", size: Some((48, 48)) },
    SlotSpec { key: "android-hdpi", group: "android", label: "hdpi", rel: "android/hdpi.png", size: Some((72, 72)) },
    SlotSpec { key: "android-xhdpi", group: "android", label: "xhdpi", rel: "android/xhdpi.png", size: Some((96, 96)) },
    SlotSpec { key: "android-xxhdpi", group: "android", label: "xxhdpi", rel: "android/xxhdpi.png", size: Some((144, 144)) },
    SlotSpec { key: "android-xxxhdpi", group: "android", label: "xxxhdpi", rel: "android/xxxhdpi.png", size: Some((192, 192)) },
];

fn icon_dir(state: &AppState) -> PathBuf {
    state.mod_root.join("assets").join("icon")
}

fn slot_by_key(key: &str) -> Option<&'static SlotSpec> {
    SLOTS.iter().find(|s| s.key == key)
}

/// Strip a `data:image/png;base64,...` prefix and base64-decode the payload.
fn decode_data_url(data_url: &str) -> Result<Vec<u8>, String> {
    let b64 = data_url
        .split_once(',')
        .map(|(_, body)| body)
        .ok_or_else(|| "invalid data URL (missing comma)".to_string())?;
    STANDARD
        .decode(b64)
        .map_err(|e| format!("invalid base64 data: {}", e))
}

/// Resize a PNG file to a small thumbnail (longest edge ≤ `max`), re-encoded
/// as a base64 data URL for the UI to render directly.
fn png_thumb_data_url(path: &Path, max: u32) -> Option<String> {
    let img = image::open(path).ok()?;
    let (w, h) = img.dimensions();
    let rgba = img.to_rgba8();
    let small = if w > max || h > max {
        let scale = (max as f32) / (w.max(h).max(1) as f32);
        let tw = ((w as f32 * scale).round().max(1.0)) as u32;
        let th = ((h as f32 * scale).round().max(1.0)) as u32;
        image::imageops::resize(&rgba, tw, th, FilterType::Lanczos3)
    } else {
        rgba
    };
    let mut buf = Vec::new();
    small
        .write_to(&mut std::io::Cursor::new(&mut buf), image::ImageFormat::Png)
        .ok()?;
    Some(format!("data:image/png;base64,{}", STANDARD.encode(buf)))
}

/// Read an image's dimensions if the file exists and decodes.
fn actual_size(path: &Path) -> Option<(u32, u32)> {
    image::open(path).ok().map(|i| i.dimensions())
}

/// Write a PNG to the slot's convention path, creating parent dirs.
fn write_png(slot: &SlotSpec, dir: &Path, rgba: &image::RgbaImage) -> Result<(), String> {
    let path = dir.join(slot.rel);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let mut file = std::fs::File::create(&path).map_err(|e| e.to_string())?;
    rgba.write_to(&mut file, image::ImageFormat::Png)
        .map_err(|e| e.to_string())
}

/// The JSON description of one slot for the frontend.
fn slot_json(slot: &SlotSpec, dir: &Path) -> Value {
    let path = dir.join(slot.rel);
    let exists = path.is_file();
    let thumb = exists.then(|| png_thumb_data_url(&path, 64)).flatten();
    json!({
        "key": slot.key,
        "group": slot.group,
        "label": slot.label,
        "relPath": slot.rel,
        "targetSize": slot.size.map(|(w, h)| json!([w, h])).unwrap_or(Value::Null),
        "exists": exists,
        "actualSize": actual_size(&path).map(|(w, h)| json!([w, h])).unwrap_or(Value::Null),
        "thumb": thumb,
        "path": path.to_string_lossy().into_owned(),
    })
}

/// Load a data-URL image and scale it to a slot (or keep as-is for window).
fn decode_resize(data_url: &str, size: Option<(u32, u32)>) -> Result<image::RgbaImage, String> {
    let bytes = decode_data_url(data_url)?;
    let img = DynamicImage::ImageRgba8(
        image::load_from_memory(&bytes)
            .map_err(|e| format!("not a decodable image (PNG expected): {}", e))?
            .to_rgba8(),
    );
    match size {
        Some((tw, th)) => Ok(image::imageops::resize(
            &img.to_rgba8(),
            tw,
            th,
            FilterType::Lanczos3,
        )),
        None => Ok(img.to_rgba8()),
    }
}

/// Full status: every slot with existence, actual size and a thumbnail.
#[tauri::command]
pub fn icon_status(state: State<AppState>) -> Value {
    let dir = icon_dir(&state);
    let slots: Vec<Value> = SLOTS.iter().map(|s| slot_json(s, &dir)).collect();
    json!({
        "iconDir": dir.to_string_lossy().into_owned(),
        "slots": slots,
    })
}

/// Set a single slot from a data URL (the picked image is resized to the
/// slot's target size). Returns the updated slot description.
#[tauri::command(rename_all = "camelCase")]
pub fn icon_set(state: State<AppState>, key: String, data_url: String) -> Result<Value, String> {
    let slot = slot_by_key(&key).ok_or_else(|| format!("unknown icon slot: {}", key))?;
    let rgba = decode_resize(&data_url, slot.size)?;
    let dir = icon_dir(&state);
    write_png(slot, &dir, &rgba)?;
    Ok(slot_json(slot, &dir))
}

/// Delete one slot's icon file.
#[tauri::command]
pub fn icon_clear(state: State<AppState>, key: String) -> Result<Value, String> {
    let slot = slot_by_key(&key).ok_or_else(|| format!("unknown icon slot: {}", key))?;
    let dir = icon_dir(&state);
    let path = dir.join(slot.rel);
    if path.is_file() {
        std::fs::remove_file(&path).map_err(|e| e.to_string())?;
    }
    Ok(slot_json(slot, &dir))
}

/// Generate every slot from one source image: the window icon is saved
/// as-is, every sized slot gets a resized copy.
#[tauri::command(rename_all = "camelCase")]
pub fn icon_generate(state: State<AppState>, data_url: String) -> Result<Value, String> {
    let src = decode_data_url(&data_url)?;
    let base = image::load_from_memory(&src)
        .map_err(|e| format!("not a decodable image (PNG expected): {}", e))?
        .to_rgba8();
    let dir = icon_dir(&state);
    for slot in SLOTS {
        let rgba = match slot.size {
            Some((tw, th)) => image::imageops::resize(&base, tw, th, FilterType::Lanczos3),
            None => base.clone(),
        };
        write_png(slot, &dir, &rgba)?;
    }
    let slots: Vec<Value> = SLOTS.iter().map(|s| slot_json(s, &dir)).collect();
    Ok(json!({ "iconDir": dir.to_string_lossy().into_owned(), "slots": slots }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slot_keys_are_unique_and_reference_known_paths() {
        let mut keys = std::collections::HashSet::new();
        for s in SLOTS {
            assert!(keys.insert(s.key), "duplicate slot key: {}", s.key);
            assert!(s.rel.ends_with(".png"));
            assert!(!s.rel.contains(".."));
        }
        assert_eq!(SLOTS.len(), 13);
    }

    #[test]
    fn decode_data_url_accepts_prefix_and_payload() {
        let bytes = b"hello";
        let url = format!("data:image/png;base64,{}", STANDARD.encode(bytes));
        assert_eq!(decode_data_url(&url).unwrap(), bytes);
    }

    #[test]
    fn decode_data_url_rejects_garbage() {
        assert!(decode_data_url("not-a-url").is_err());
        assert!(decode_data_url("data:image/png;base64,%%%").is_err());
    }

    #[test]
    fn decode_resize_scales_to_target() {
        let rgba = image::RgbaImage::from_pixel(64, 64, image::Rgba([255u8, 0, 0, 255]));
        let mut buf = Vec::new();
        rgba.write_to(&mut std::io::Cursor::new(&mut buf), image::ImageFormat::Png).unwrap();
        let url = format!("data:image/png;base64,{}", STANDARD.encode(buf));

        let out = decode_resize(&url, Some((16, 16))).unwrap();
        assert_eq!(out.dimensions(), (16, 16));

        let win = decode_resize(&url, None).unwrap();
        assert_eq!(win.dimensions(), (64, 64));
    }

    #[test]
    fn write_and_thumb_round_trip() {
        // A real write to a temp dir exercising write_png + slot_json + the
        // thumbnail data URL that the UI renders.
        let dir = std::env::temp_dir().join(format!("kdt-gui-icon-test-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();

        let slot = slot_by_key("win-32").unwrap();
        let rgba = image::RgbaImage::from_pixel(32, 32, image::Rgba([0u8, 128, 255, 255]));
        write_png(slot, &dir, &rgba).unwrap();

        let json = slot_json(slot, &dir);
        assert_eq!(json["exists"], true);
        assert_eq!(json["actualSize"], json!([32, 32]));
        let thumb = json["thumb"].as_str().unwrap();
        assert!(thumb.starts_with("data:image/png;base64,"), "thumb is a data URL");

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn generate_all_matches_the_build_convention() {
        // Simulate icon_generate on a 256×256 source: every slot must land at
        // the size thrash-machine's build scripts expect, and the window icon
        // must be saved as-is (no resize).
        let dir = std::env::temp_dir().join(format!("kdt-gui-gen-test-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();

        let base = image::RgbaImage::from_pixel(256, 256, image::Rgba([255u8, 0, 255, 255]));
        for slot in SLOTS {
            let rgba = match slot.size {
                Some((tw, th)) => image::imageops::resize(&base, tw, th, FilterType::Lanczos3),
                None => base.clone(),
            };
            write_png(slot, &dir, &rgba).unwrap();
        }

        let expect = |key: &str, w: u32, h: u32| {
            let s = slot_by_key(key).unwrap();
            let path = dir.join(s.rel);
            let dims = image::open(&path).unwrap().dimensions();
            assert_eq!(dims, (w, h), "slot {} should be {}×{}", key, w, h);
        };
        expect("window", 256, 256); // as-is
        for (k, n) in [("win-16", 16), ("win-32", 32), ("win-48", 48), ("win-64", 64), ("win-128", 128), ("win-256", 256)] {
            expect(k, n, n);
        }
        for (k, n) in [("android-ldpi", 36), ("android-mdpi", 48), ("android-hdpi", 72),
                       ("android-xhdpi", 96), ("android-xxhdpi", 144), ("android-xxxhdpi", 192)] {
            expect(k, n, n);
        }

        let _ = std::fs::remove_dir_all(&dir);
    }
}

//! Hierarchical MIDI mapping persistence.
//!
//! Layers (lowest → highest precedence):
//!   1. Factory device preset  — from midi_presets, compiled in
//!   2. User device overrides  — ~/Library/Application Support/Forma/midi-mappings/<slug>.json
//!
//! When no device is connected, a plain user.json is used instead.

use directories::ProjectDirs;
use forma_control::ParamId;
use std::collections::HashMap;
use std::path::PathBuf;

fn user_dir() -> Option<PathBuf> {
    ProjectDirs::from("com", "francescoventura", "Forma")
        .map(|d| d.data_dir().join("midi-mappings"))
}

/// Normalize a device name to a safe filename slug.
/// "Arturia KeyLab mkIII MIDI In" → "arturia-keylab-mkiii-midi-in"
pub fn device_slug(name: &str) -> String {
    let mut slug = String::new();
    let mut last_sep = true;
    for ch in name.to_lowercase().chars() {
        if ch.is_alphanumeric() {
            slug.push(ch);
            last_sep = false;
        } else if !last_sep {
            slug.push('-');
            last_sep = true;
        }
    }
    slug.trim_end_matches('-').to_string()
}

fn read_json(path: PathBuf) -> HashMap<u8, ParamId> {
    std::fs::read_to_string(path)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

/// Load and merge all layers for a connected device.
pub fn load_for_device(device_name: &str) -> HashMap<u8, ParamId> {
    // Layer 1: factory preset
    let mut merged = crate::midi_presets::bindings_for_device(device_name);

    // Layer 2: user device overrides
    if let Some(dir) = user_dir() {
        let user = read_json(dir.join(format!("{}.json", device_slug(device_name))));
        merged.extend(user);
    }

    merged
}

/// Save the user device layer (layer 2). Factory layer is never touched.
pub fn save_for_device(device_name: &str, bindings: &HashMap<u8, ParamId>) {
    let Some(dir) = user_dir() else { return };
    let _ = std::fs::create_dir_all(&dir);
    if let Ok(json) = serde_json::to_string_pretty(bindings) {
        let _ = std::fs::write(dir.join(format!("{}.json", device_slug(device_name))), json);
    }
}

/// Load bindings when no MIDI device is connected.
pub fn load_no_device() -> HashMap<u8, ParamId> {
    user_dir()
        .map(|d| read_json(d.join("user.json")))
        .unwrap_or_default()
}

/// Save bindings when no MIDI device is connected.
pub fn save_no_device(bindings: &HashMap<u8, ParamId>) {
    let Some(dir) = user_dir() else { return };
    let _ = std::fs::create_dir_all(&dir);
    if let Ok(json) = serde_json::to_string_pretty(bindings) {
        let _ = std::fs::write(dir.join("user.json"), json);
    }
}

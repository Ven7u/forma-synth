//! Hierarchical layout persistence.
//!
//! Live state  → ~/Library/Application Support/Forma/layouts/current.json
//! User saves  → ~/Library/Application Support/Forma/layouts/user/<name>.json
//! Factory     → defined in ui::layout (compiled in, read-only)

use crate::ui::layout::LayoutState;
use directories::ProjectDirs;
use std::path::PathBuf;

fn layouts_dir() -> Option<PathBuf> {
    ProjectDirs::from("com", "francescoventura", "Forma").map(|d| d.data_dir().join("layouts"))
}

fn user_dir() -> Option<PathBuf> {
    layouts_dir().map(|d| d.join("user"))
}

/// Load the last active layout. Falls back to `LayoutState::default()`.
pub fn load_current() -> LayoutState {
    layouts_dir()
        .map(|d| d.join("current.json"))
        .and_then(|p| std::fs::read_to_string(p).ok())
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

/// Persist the current active layout.
pub fn save_current(state: &LayoutState) {
    let Some(dir) = layouts_dir() else { return };
    let _ = std::fs::create_dir_all(&dir);
    if let Ok(json) = serde_json::to_string_pretty(state) {
        let _ = std::fs::write(dir.join("current.json"), json);
    }
}

/// List all user-saved layout names (filename stems, sorted).
pub fn list_user_layouts() -> Vec<String> {
    let Some(dir) = user_dir() else { return vec![] };
    let Ok(entries) = std::fs::read_dir(&dir) else {
        return vec![];
    };
    let mut names: Vec<String> = entries
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().is_some_and(|x| x == "json"))
        .filter_map(|e| {
            e.path()
                .file_stem()
                .and_then(|s| s.to_str())
                .map(|s| s.to_string())
        })
        .collect();
    names.sort();
    names
}

/// Save the current state under a user-chosen name.
pub fn save_named(name: &str, state: &LayoutState) {
    let Some(dir) = user_dir() else { return };
    let _ = std::fs::create_dir_all(&dir);
    let filename = sanitize(name) + ".json";
    if let Ok(json) = serde_json::to_string_pretty(state) {
        let _ = std::fs::write(dir.join(filename), json);
    }
}

/// Load a user-saved layout by name. Returns `None` if not found.
pub fn load_named(name: &str) -> Option<LayoutState> {
    let dir = user_dir()?;
    let path = dir.join(sanitize(name) + ".json");
    serde_json::from_str(&std::fs::read_to_string(path).ok()?).ok()
}

/// Delete a user-saved layout by name.
pub fn delete_named(name: &str) {
    let Some(dir) = user_dir() else { return };
    let _ = std::fs::remove_file(dir.join(sanitize(name) + ".json"));
}

/// Strip characters that are unsafe in filenames.
fn sanitize(name: &str) -> String {
    name.chars()
        .map(|c| {
            if c.is_alphanumeric() || c == ' ' || c == '-' || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect::<String>()
        .trim()
        .to_string()
}

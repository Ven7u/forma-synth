//! Patch re-export + factory-preset loader.
//!
//! The `Patch` struct itself lives in [`forma_engine::patch`] so that any
//! frontend (egui, Bevy, Swift, web, DAW plugin) can round-trip patches
//! through the engine handle without re-implementing the schema. This
//! module keeps the `crate::patch::Patch` import path working and owns the
//! UI-side concern of loading the factory preset library.

pub use forma_engine::Patch;

use include_dir::{include_dir, Dir};

/// All factory patches embedded into the binary at compile time. Guarantees
/// the patch library is populated regardless of install method (cargo install
/// copies only the binary, with no associated resources on disk).
static EMBEDDED_PATCHES: Dir<'_> = include_dir!("$CARGO_MANIFEST_DIR/../../assets/patches");

fn collect_patch_files(dir: &std::path::Path, out: &mut Vec<std::path::PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for ent in entries.flatten() {
        let p = ent.path();
        if p.is_dir() {
            collect_patch_files(&p, out);
        } else if p
            .extension()
            .and_then(|e| e.to_str())
            .map(|e| e.eq_ignore_ascii_case("json"))
            .unwrap_or(false)
        {
            out.push(p);
        }
    }
}

/// Locate an on-disk patches directory, trying (in order):
///   1. `assets/patches` relative to CWD — works with `cargo run` and lets devs
///      edit patch files and reload them without recompiling.
///   2. `../Resources/assets/patches` relative to the canonicalized executable —
///      works inside a `.app` bundle (exe at `Contents/MacOS/`, resources at
///      `Contents/Resources/`) and inside a Homebrew keg (exe at `<prefix>/bin/`,
///      resources at `<prefix>/Resources/`). Canonicalisation resolves PATH
///      symlinks like `/usr/local/bin/forma → Cellar/forma/<ver>/bin/forma`.
///
/// Returns `None` for `cargo install` and similar binary-only installs — the
/// caller falls back to the embedded patches in that case.
fn on_disk_patches_dir() -> Option<std::path::PathBuf> {
    let cwd_path = std::path::Path::new("assets/patches");
    if cwd_path.is_dir() {
        return Some(cwd_path.to_path_buf());
    }
    let exe = std::env::current_exe().ok()?;
    let real_exe = std::fs::canonicalize(&exe).unwrap_or(exe);
    let bundle_path = real_exe
        .parent() // Contents/MacOS or <prefix>/bin
        .and_then(|p| p.parent()) // Contents or <prefix>
        .map(|p| p.join("Resources").join("assets").join("patches"))?;
    bundle_path.is_dir().then_some(bundle_path)
}

fn parse_embedded_patches() -> Vec<Patch> {
    let mut entries: Vec<&include_dir::File<'_>> = EMBEDDED_PATCHES
        .find("**/*.json")
        .ok()
        .into_iter()
        .flatten()
        .filter_map(|e| e.as_file())
        .collect();
    entries.sort_by_key(|f| f.path());
    entries
        .into_iter()
        .filter_map(|f| f.contents_utf8())
        .filter_map(|s| serde_json::from_str(s).ok())
        .collect()
}

pub fn default_patches() -> Vec<Patch> {
    if let Some(dir) = on_disk_patches_dir() {
        let mut files = Vec::new();
        collect_patch_files(&dir, &mut files);
        files.sort();
        let parsed: Vec<Patch> = files
            .iter()
            .filter_map(|p| std::fs::read_to_string(p).ok())
            .filter_map(|s| serde_json::from_str(&s).ok())
            .collect();
        if !parsed.is_empty() {
            return parsed;
        }
    }
    parse_embedded_patches()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every on-disk preset in `assets/patches/**/*.json` must continue to
    /// parse via the migrated `Patch` struct (now in `forma-engine`).
    /// Catches serde-field drift during the Stage 3 move.
    #[test]
    fn embedded_patches_load_at_least_one() {
        // Guards against breakage in the include_dir glob, missing assets at
        // build time, or schema drift that makes every patch fail to parse.
        let parsed = parse_embedded_patches();
        assert!(
            !parsed.is_empty(),
            "no embedded patches parsed — check the include_dir path and patch JSON schema",
        );
    }

    #[test]
    fn every_bundled_patch_json_deserialises() {
        // Walk relative to the workspace root; `cargo test` sets CWD there.
        let root = std::env::var_os("CARGO_MANIFEST_DIR")
            .map(std::path::PathBuf::from)
            .and_then(|p| p.parent().map(|w| w.parent().unwrap().to_path_buf()))
            .unwrap_or_else(|| std::path::PathBuf::from("."));
        let patches_dir = root.join("assets/patches");
        if !patches_dir.exists() {
            // Skip silently on environments without assets.
            return;
        }

        let mut files = Vec::new();
        collect_patch_files(&patches_dir, &mut files);
        assert!(
            !files.is_empty(),
            "no patch files under {}",
            patches_dir.display()
        );

        let mut failures = Vec::new();
        for f in &files {
            let body = match std::fs::read_to_string(f) {
                Ok(s) => s,
                Err(e) => {
                    failures.push(format!("{}: read error {}", f.display(), e));
                    continue;
                }
            };
            if let Err(e) = serde_json::from_str::<Patch>(&body) {
                failures.push(format!("{}: {}", f.display(), e));
            }
        }
        assert!(
            failures.is_empty(),
            "{}/{} patch file(s) failed to deserialise:\n{}",
            failures.len(),
            files.len(),
            failures.join("\n"),
        );
    }
}

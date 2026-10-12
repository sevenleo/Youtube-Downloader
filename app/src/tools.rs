//! Discovery of yt-dlp/ffmpeg binaries. Path resolution.

use anyhow::{Context, Result};
use std::path::{Path, PathBuf};

/// Resolved paths of binaries used by the Download tab.
#[derive(Clone, Debug)]
pub struct ToolPaths {
    pub ytdlp: PathBuf,
    pub ffmpeg: PathBuf,
}

fn exe_dir() -> Option<PathBuf> {
    std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|d| d.to_path_buf()))
}

/// Searches for a binary (first matching name wins) in order:
/// 1. `bin/` directory next to the running executable (standard production setup);
/// 2. `bin/` ascending up to 5 parent levels from the executable (for `cargo run` in target/debug);
/// 3. System `PATH` (fallback matching the .bat scripts).
fn find_binary(names: &[&str]) -> Option<PathBuf> {
    // (1) + (2): level 0 = exe dir; ascends up to 5 levels.
    if let Some(mut dir) = exe_dir() {
        for _ in 0..=5 {
            for name in names {
                let candidate = dir.join("bin").join(name);
                if candidate.is_file() {
                    return Some(candidate);
                }
            }
            match dir.parent() {
                Some(parent) => dir = parent.to_path_buf(),
                None => break,
            }
        }
    }
    // (3): PATH (with and without .exe extension).
    if let Ok(path_var) = std::env::var("PATH") {
        for entry in std::env::split_paths(&path_var) {
            // Strip quotes that some installers leave in PATH entries.
            let entry_str = entry.to_string_lossy();
            let trimmed = entry_str.trim_matches('"');
            let entry = Path::new(trimmed);
            for name in names {
                let candidate = entry.join(name);
                if candidate.is_file() {
                    return Some(candidate);
                }
            }
        }
    }
    None
}

/// Locates yt-dlp.exe and ffmpeg.exe. Order (first match wins):
/// 1. `bin/` directory next to running executable (standard case);
/// 2. `bin/` at workspace root (ascending parent dirs from current_exe, for `cargo run`);
/// 3. System `PATH` (fallback matching .bat scripts).
pub fn locate() -> Result<ToolPaths> {
    let ytdlp = find_binary(&["yt-dlp.exe", "yt-dlp"])
        .ok_or_else(|| anyhow::anyhow!("ERROR: yt-dlp was not found in bin directory or in PATH."))?;
    let ffmpeg = find_binary(&["ffmpeg.exe", "ffmpeg"])
        .ok_or_else(|| anyhow::anyhow!("ERROR: ffmpeg was not found in bin directory or in PATH."))?;
    Ok(ToolPaths { ytdlp, ffmpeg })
}

/// Canonical project `bin` folder (next to the executable); created if it does not exist.
/// Used by the Update tab, including when binaries have not yet been downloaded.
pub fn ensure_bin_dir() -> Result<PathBuf> {
    // (1): ascend directories from exe looking for existing folder containing `bin/`.
    if let Some(mut dir) = exe_dir() {
        for _ in 0..=5 {
            let bin = dir.join("bin");
            if bin.is_dir() {
                // Ensure existence (create if needed) and return.
                std::fs::create_dir_all(&bin)
                    .with_context(|| format!("could not create {}", bin.display()))?;
                return Ok(bin);
            }
            match dir.parent() {
                Some(parent) => dir = parent.to_path_buf(),
                None => break,
            }
        }
    }
    // (2): `<exe-dir>/bin` + create_dir_all.
    let fallback = exe_dir()
        .map(|d| d.join("bin"))
        .unwrap_or_else(|| PathBuf::from("bin"));
    std::fs::create_dir_all(&fallback)
        .with_context(|| format!("could not create {}", fallback.display()))?;
    Ok(fallback)
}

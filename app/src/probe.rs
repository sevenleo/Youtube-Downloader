//! URL analysis via `yt-dlp --dump-single-json`. Parity with download.bat.

use anyhow::{Context, Result};
use std::path::Path;

use crate::process::run_capture_stdout;

/// An H.264 resolution actually available in the video.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct H264Resolution {
    pub height: u32,
    /// Largest width among formats with this height.
    pub width: u32,
}

/// Video metadata required for UI display and download execution.
#[derive(Clone, Debug)]
pub struct VideoInfo {
    pub title: String,
    /// Duration in seconds (0 if unknown).
    pub duration_secs: u64,
    /// Thumbnail URL (from JSON `thumbnail` field), if available.
    pub thumbnail_url: Option<String>,
    /// H.264 resolutions (vcodec starting with "avc1"), sorted by height.
    pub resolutions: Vec<H264Resolution>,
}

impl VideoInfo {
}

/// Positive integer parsed from JSON (yt-dlp emits ints; accepts float/string for robustness,
/// similar to `[int]` cast in .bat).
fn json_u32_positive(v: &serde_json::Value) -> Option<u32> {
    if let Some(n) = v.as_u64() {
        return u32::try_from(n).ok().filter(|&x| x > 0);
    }
    if let Some(n) = v.as_i64() {
        return if n > 0 { u32::try_from(n).ok() } else { None };
    }
    if let Some(n) = v.as_f64() {
        return if n.is_finite() && n > 0.0 {
            Some(n as u32).filter(|&x| x > 0)
        } else {
            None
        };
    }
    if let Some(s) = v.as_str() {
        if let Ok(n) = s.trim().parse::<u32>() {
            return if n > 0 { Some(n) } else { None };
        }
    }
    None
}

/// Runs `yt-dlp --force-ipv4 --dump-single-json --skip-download --no-warnings <url>`
/// and extracts title/duration/thumbnail/H.264 resolutions. Errors mapped as in .bat
/// (empty title = ERROR_TITLE; no H.264 resolution = ERROR_FORMATS).
pub fn analyze(ytdlp: &Path, url: &str) -> Result<VideoInfo> {
    if url.trim().is_empty() {
        anyhow::bail!("ERROR: no link was provided.");
    }
    let args: Vec<String> = [
        "--force-ipv4",
        "--dump-single-json",
        "--skip-download",
        "--no-warnings",
        url,
    ]
    .iter()
    .map(|s| s.to_string())
    .collect();

    let out = run_capture_stdout(ytdlp, &args)
        .context("failed to execute yt-dlp --dump-single-json")?;
    let v: serde_json::Value = serde_json::from_str(&out)
        .map_err(|_| anyhow::anyhow!("ERROR: could not retrieve title."))?;

    let title = v
        .get("title")
        .and_then(|t| t.as_str())
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .ok_or_else(|| anyhow::anyhow!("ERROR: could not retrieve title."))?;

    // Formats with vcodec starting with "avc1" and height/width > 0,
    // grouped by height (maximum width of group), sorted by height.
    let mut by_height: std::collections::BTreeMap<u32, u32> = std::collections::BTreeMap::new();
    if let Some(formats) = v.get("formats").and_then(|f| f.as_array()) {
        for f in formats {
            let is_h264 = f
                .get("vcodec")
                .and_then(|c| c.as_str())
                .map(|c| c.starts_with("avc1"))
                .unwrap_or(false);
            if !is_h264 {
                continue;
            }
            let height = f.get("height").and_then(json_u32_positive);
            let width = f.get("width").and_then(json_u32_positive);
            if let (Some(h), Some(w)) = (height, width) {
                by_height
                    .entry(h)
                    .and_modify(|max_w| *max_w = (*max_w).max(w))
                    .or_insert(w);
            }
        }
    }
    let resolutions: Vec<H264Resolution> = by_height
        .into_iter()
        .map(|(height, width)| H264Resolution { height, width })
        .collect();
    if resolutions.is_empty() {
        anyhow::bail!("ERROR: no H.264 resolutions found for this video.");
    }

    let duration_secs = v
        .get("duration")
        .map(|d| {
            if let Some(n) = d.as_u64() {
                n
            } else if let Some(n) = d.as_i64() {
                n.max(0) as u64
            } else if let Some(n) = d.as_f64() {
                if n.is_finite() && n > 0.0 {
                    n as u64
                } else {
                    0
                }
            } else {
                0
            }
        })
        .unwrap_or(0);

    // Field `thumbnail`; fallback: last entry in `thumbnails` with valid url.
    let thumbnail_url = v
        .get("thumbnail")
        .and_then(|t| t.as_str())
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())
        .or_else(|| {
            v.get("thumbnails")
                .and_then(|t| t.as_array())
                .and_then(|arr| {
                    arr.iter()
                        .rev()
                        .filter_map(|t| t.get("url").and_then(|u| u.as_str()))
                        .map(str::trim)
                        .find(|s| !s.is_empty())
                        .map(|s| s.to_string())
                })
        });

    Ok(VideoInfo {
        title: title.to_string(),
        duration_secs,
        thumbnail_url,
        resolutions,
    })
}

/// Downloads thumbnail (for UI display). Failure does not interrupt the main workflow.
pub fn fetch_thumbnail(client: &reqwest::blocking::Client, url: &str) -> Result<Vec<u8>> {
    let bytes = client
        .get(url)
        .send()
        .context("failed to download thumbnail")?
        .error_for_status()
        .context("thumbnail returned error status")?
        .bytes()
        .context("failed to read thumbnail bytes")?;
    Ok(bytes.to_vec())
}

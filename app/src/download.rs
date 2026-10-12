//! Download workflow equivalent to download.bat. Parity with download.bat.

use anyhow::Result;
use std::path::PathBuf;
use std::sync::atomic::AtomicBool;
use std::sync::mpsc::Sender;

use crate::log::DownloadLog;
use crate::process::{run_streamed, ProcLine};
use crate::state::{DownloadParams, DownloadStage, UiEvent, VideoMode};
use crate::tools::ToolPaths;

/// Title -> safe filename (invalid Windows characters -> `_`).
pub fn sanitize_filename(title: &str) -> String {
    let safe: String = title
        .chars()
        .map(|c| {
            if matches!(c, '<' | '>' | ':' | '"' | '/' | '\\' | '|' | '?' | '*')
                || c.is_control()
            {
                '_'
            } else {
                c
            }
        })
        .collect();
    let trimmed = safe.trim().to_string();
    if trimmed.is_empty() {
        "video".to_string()
    } else {
        trimmed
    }
}

/// Builds the format selector string for height `height`.
/// E.g.: `bv*[height=1080][vcodec^=avc1]+ba[acodec=opus]/bv*[height=1080]+ba/...`
///
/// NOTE: without cmd escaping (`%%` -> `%`, `^^` -> `^`); in Rust
/// arguments are passed literally to the child process without a shell.
pub fn format_string(height: u32) -> String {
    format!(
        "bv*[height={h}][vcodec^=avc1]+ba[acodec=opus]/bv*[height={h}]+ba/b[height={h}]/bv*[height<={h}]+ba/best[height<={h}]",
        h = height
    )
}

/// RAII guard for temporary folder: deletes `%TEMP%\yt-dlp-video-*` on Drop,
/// ensuring cleanup even upon error or cancellation.
struct TempGuard {
    path: PathBuf,
}

impl Drop for TempGuard {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.path);
    }
}

/// Records a process line in download.log AND in the UI; extracts download `%`.
fn handle_line(
    dlog: &mut DownloadLog,
    emit: &Sender<UiEvent>,
    progress_re: &regex::Regex,
    line: ProcLine,
) {
    let _ = dlog.line(&line.text);
    let _ = emit.send(UiEvent::LogLine(line.text.clone()));
    if let Some(caps) = progress_re.captures(&line.text) {
        if let Ok(p) = caps[1].parse::<f32>() {
            let _ = emit.send(UiEvent::DownloadProgress {
                percent: p.clamp(0.0, 100.0),
            });
        }
    }
}

/// User-friendly error styled after ERROR_* from .bat; technical detail goes to log.
fn fail(dlog: &mut DownloadLog, friendly: &str, detail: impl std::fmt::Display) -> anyhow::Error {
    let _ = dlog.line(&format!("{friendly} Detail: {detail}"));
    anyhow::anyhow!("{friendly}")
}

/// Executes the full download workflow (download -> optional clip cut -> mkv/mp4 -> final file),
/// emitting `UiEvent`s for progress, stage, and log output. Can be cancelled via `cancel`.
/// Returns final file path.
pub fn run_download(
    tools: &ToolPaths,
    params: &DownloadParams,
    emit: &Sender<UiEvent>,
    cancel: &AtomicBool,
) -> Result<PathBuf> {
    use std::sync::atomic::Ordering;
    use std::time::{SystemTime, UNIX_EPOCH};

    let cancelled = || cancel.load(Ordering::SeqCst);

    // download.log recreated from scratch (truncate) on each download, matching .bat.
    let mut dlog = DownloadLog::create(&params.output_dir)
        .map_err(|e| anyhow::anyhow!("ERROR: please select a valid and writable destination folder. ({e})"))?;

    // Initial binary existence checks.
    if !tools.ytdlp.is_file() {
        let msg = "ERROR: yt-dlp was not found in bin directory or in PATH.";
        let _ = dlog.line(msg);
        anyhow::bail!("{msg}");
    }
    if !tools.ffmpeg.is_file() {
        let msg = "ERROR: ffmpeg was not found in bin directory or in PATH.";
        let _ = dlog.line(msg);
        anyhow::bail!("{msg}");
    }

    // Metadata in log (matching .bat: URL/TITLE/RESOLUTION/FORMAT/START/END).
    let _ = dlog.line(&format!("URL: {}", params.url));
    let _ = dlog.line(&format!("TITLE: {}", params.video_title));
    let _ = dlog.line(&format!("RESOLUTION: {}p", params.height));
    let _ = dlog.line(&format!("FORMAT: {}", params.container.ext()));
    if let VideoMode::Clip { start, end } = &params.mode {
        if start.trim().is_empty() {
            let msg = "ERROR: start time cannot be empty.";
            let _ = dlog.line(msg);
            anyhow::bail!("{msg}");
        }
        if end.trim().is_empty() {
            let msg = "ERROR: end time cannot be empty.";
            let _ = dlog.line(msg);
            anyhow::bail!("{msg}");
        }
        let _ = dlog.line(&format!("START: {start}"));
        let _ = dlog.line(&format!("END: {end}"));
    }

    // Temporary folder %TEMP%\yt-dlp-video-<nanos>.
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let temp_dir = std::env::temp_dir().join(format!("yt-dlp-video-{nanos}"));
    std::fs::create_dir_all(&temp_dir).map_err(|e| {
        fail(
            &mut dlog,
            "ERROR: could not create temporary directory.",
            e,
        )
    })?;
    let _temp_guard = TempGuard {
        path: temp_dir.clone(),
    };
    let _ = dlog.line(&format!("TEMP_DIR: {}", temp_dir.display()));

    let progress_re = regex::Regex::new(r"\[download\]\s+(\d+(?:\.\d+)?)%")
        .map_err(|e| fail(&mut dlog, "ERROR: internal error (regex).", e))?;

    // --- yt-dlp download ---
    let _ = emit.send(UiEvent::StageChanged(DownloadStage::Downloading));
    let out_tpl = format!(r"{}\source.%(ext)s", temp_dir.display());
    let ytdlp_args: Vec<String> = vec![
        "--force-ipv4",
        "--retries",
        "20",
        "--fragment-retries",
        "20",
        "--retry-sleep",
        "2",
        "--newline",
        "--print",
        "RESOLVED_FORMAT: %(format_id)s",
        "--no-simulate",
        "--verbose",
        "-f",
        &format_string(params.height),
        "--merge-output-format",
        "mkv",
        "--remux-video",
        "mkv",
        "-o",
        &out_tpl,
        &params.url,
    ]
    .into_iter()
    .map(|s| s.to_string())
    .collect();

    let mut resolved_format: Option<String> = None;
    let dl_code = run_streamed(
        &tools.ytdlp,
        &ytdlp_args,
        &mut |line: ProcLine| {
            if let Some(id) = line.text.strip_prefix("RESOLVED_FORMAT:") {
                resolved_format = Some(id.trim().to_string());
            }
            handle_line(&mut dlog, emit, &progress_re, line);
        },
        cancel,
    )
    .map_err(|e| fail(&mut dlog, "ERROR: download failed.", format!("yt-dlp: {e}")))?;
    if cancelled() {
        let msg = "Download cancelled by user.";
        let _ = dlog.line(msg);
        anyhow::bail!("{msg}");
    }
    if dl_code != 0 {
        return Err(fail(
            &mut dlog,
            "ERROR: download failed. See download.log for details.",
            format!("exit code {dl_code}"),
        ));
    }
    if resolved_format.is_none() {
        let _ = dlog.line("Warning: resolved format not detected in yt-dlp output.");
    }

    // Require source.mkv.
    let source = temp_dir.join("source.mkv");
    if !source.is_file() {
        return Err(fail(
            &mut dlog,
            "ERROR: source.mkv was not found.",
            "source.mkv nonexistent after yt-dlp",
        ));
    }
    let _ = dlog.line("DOWNLOAD COMPLETED");

    // --- Clip cutting (clip mode only) ---
    let mut input = source.clone();
    if let VideoMode::Clip { start, end } = &params.mode {
        let _ = emit.send(UiEvent::StageChanged(DownloadStage::Cutting));
        let clip_path = temp_dir.join("clip.mkv");
        let _ = dlog.line(&format!("Clip: {start} to {end}"));
        let source_s = source.to_string_lossy().into_owned();
        let clip_s = clip_path.to_string_lossy().into_owned();
        let cut_args: Vec<String> = vec![
            "-y", "-ss", start, "-to", end, "-i", &source_s, "-map", "0", "-c", "copy",
            &clip_s,
        ]
        .into_iter()
        .map(|s| s.to_string())
        .collect();
        let cut_code = run_streamed(
            &tools.ffmpeg,
            &cut_args,
            &mut |line: ProcLine| handle_line(&mut dlog, emit, &progress_re, line),
            cancel,
        )
        .map_err(|e| fail(&mut dlog, "ERROR: failed to cut clip.", format!("ffmpeg: {e}")))?;
        if cancelled() {
            let msg = "Download cancelled by user.";
            let _ = dlog.line(msg);
            anyhow::bail!("{msg}");
        }
        if cut_code != 0 || !clip_path.is_file() {
            return Err(fail(
                &mut dlog,
                "ERROR: failed to cut clip.",
                format!("exit code {cut_code}"),
            ));
        }
        input = clip_path;
    }

    // --- Final output file ---
    let final_name = final_file_name(params);
    let final_path = params.output_dir.join(&final_name);
    let _ = dlog.line(&format!("FINAL: {}", final_path.display()));

    match params.container {
        crate::state::ContainerFormat::Mkv => {
            // MKV = direct copy of intermediate file.
            std::fs::copy(&input, &final_path).map_err(|e| {
                fail(
                    &mut dlog,
                    "ERROR: final file was not created.",
                    format!("copy: {e}"),
                )
            })?;
        }
        crate::state::ContainerFormat::Mp4 => {
            let _ = emit.send(UiEvent::StageChanged(DownloadStage::Converting));
            let input_s = input.to_string_lossy().into_owned();
            let final_s = final_path.to_string_lossy().into_owned();
            let conv_args: Vec<String> = vec![
                "-y",
                "-i",
                &input_s,
                "-map",
                "0:v:0",
                "-map",
                "0:a:0",
                "-c:v",
                "copy",
                "-c:a",
                "aac",
                "-b:a",
                "320k",
                "-movflags",
                "+faststart",
                &final_s,
            ]
            .into_iter()
            .map(|s| s.to_string())
            .collect();
            let conv_code = run_streamed(
                &tools.ffmpeg,
                &conv_args,
                &mut |line: ProcLine| handle_line(&mut dlog, emit, &progress_re, line),
                cancel,
            )
            .map_err(|e| {
                fail(
                    &mut dlog,
                    "ERROR: failed to convert to MP4.",
                    format!("ffmpeg: {e}"),
                )
            })?;
            if cancelled() {
                let msg = "Download cancelled by user.";
                let _ = dlog.line(msg);
                anyhow::bail!("{msg}");
            }
            if conv_code != 0 {
                return Err(fail(
                    &mut dlog,
                    "ERROR: failed to convert to MP4.",
                    format!("exit code {conv_code}"),
                ));
            }
        }
    }

    let _ = emit.send(UiEvent::StageChanged(DownloadStage::Finalizing));
    if !final_path.is_file() {
        return Err(fail(
            &mut dlog,
            "ERROR: final file was not created.",
            "final file nonexistent",
        ));
    }

    let _ = std::fs::remove_dir_all(&temp_dir);
    dlog.finish()?;
    Ok(final_path)
}

/// Builds final filename: `TITLE - full.ext` or
/// `TITLE - clip HH-MM-SS-HH-MM-SS.ext` (colons become hyphens).
pub fn final_file_name(params: &DownloadParams) -> String {
    let safe = sanitize_filename(&params.video_title);
    let ext = params.container.ext();
    match &params.mode {
        VideoMode::Full => format!("{safe} - full.{ext}"),
        VideoMode::Clip { start, end } => {
            let s = start.replace(':', "-");
            let e = end.replace(':', "-");
            format!("{safe} - clip {s}-{e}.{ext}")
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::{ContainerFormat, DownloadParams, VideoMode};

    fn params(title: &str, container: ContainerFormat, mode: VideoMode) -> DownloadParams {
        DownloadParams {
            url: "https://example.com/video".to_string(),
            output_dir: PathBuf::from("C:\\out"),
            height: 1080,
            mode,
            container,
            video_title: title.to_string(),
        }
    }

    #[test]
    fn sanitize_replaces_invalid_chars() {
        assert_eq!(
            sanitize_filename("a<b>c:d\"e/f\\g|h?i*j"),
            "a_b_c_d_e_f_g_h_i_j"
        );
    }

    #[test]
    fn sanitize_replaces_controls_and_trims() {
        assert_eq!(sanitize_filename("a\x01b\x1fc"), "a_b_c");
        assert_eq!(sanitize_filename("  hello world  "), "hello world");
    }

    #[test]
    fn sanitize_empty_becomes_video() {
        assert_eq!(sanitize_filename(""), "video");
        assert_eq!(sanitize_filename("   "), "video");
        assert_eq!(sanitize_filename("<>:\"/\\|?*"), "_________");
    }

    #[test]
    fn sanitize_keeps_normal_titles() {
        assert_eq!(
            sanitize_filename("Big Buck Bunny (4K) - Test!"),
            "Big Buck Bunny (4K) - Test!"
        );
    }

    #[test]
    fn format_string_matches_bat() {
        assert_eq!(
            format_string(1080),
            "bv*[height=1080][vcodec^=avc1]+ba[acodec=opus]/bv*[height=1080]+ba/b[height=1080]/bv*[height<=1080]+ba/best[height<=1080]"
        );
        assert_eq!(
            format_string(720),
            "bv*[height=720][vcodec^=avc1]+ba[acodec=opus]/bv*[height=720]+ba/b[height=720]/bv*[height<=720]+ba/best[height<=720]"
        );
    }

    #[test]
    fn final_name_full() {
        let p = params("My Video", ContainerFormat::Mp4, VideoMode::Full);
        assert_eq!(final_file_name(&p), "My Video - full.mp4");
        let p = params("My Video", ContainerFormat::Mkv, VideoMode::Full);
        assert_eq!(final_file_name(&p), "My Video - full.mkv");
    }

    #[test]
    fn final_name_clip_colons_become_hyphens() {
        let p = params(
            "My Video",
            ContainerFormat::Mkv,
            VideoMode::Clip {
                start: "00:01:02".to_string(),
                end: "03:04".to_string(),
            },
        );
        assert_eq!(final_file_name(&p), "My Video - clip 00-01-02-03-04.mkv");
    }

    #[test]
    fn final_name_clip_sanitizes_title() {
        let p = params(
            "A/B:C",
            ContainerFormat::Mp4,
            VideoMode::Clip {
                start: "00:00:10".to_string(),
                end: "00:00:20".to_string(),
            },
        );
        assert_eq!(
            final_file_name(&p),
            "A_B_C - clip 00-00-10-00-00-20.mp4"
        );
    }
}

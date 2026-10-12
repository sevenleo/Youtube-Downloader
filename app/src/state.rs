//! Interface contract between UI and application core.

use std::path::PathBuf;

/// Binary managed by the Update tab.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BinTarget {
    YtDlp,
    FFmpeg,
}

/// Status of a binary (badge in UI). Inference rules match update.bat.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BadgeState {
    /// Green: installed == latest.
    UpToDate,
    /// Yellow: installed != latest.
    UpdateAvailable,
    /// Red: not installed and a version is available.
    NotInstalled,
    /// Gray: unable to verify.
    Unknown,
}

/// Installed version + latest version + badge status for a binary.
#[derive(Clone, Debug)]
pub struct BinaryStatus {
    pub installed: Option<String>,
    pub latest: Option<String>,
    pub badge: BadgeState,
}

/// Download content mode: full video or clip with timestamps ("HH:MM:SS" or "MM:SS").
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum VideoMode {
    Full,
    Clip { start: String, end: String },
}

/// Output container format.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ContainerFormat {
    Mkv,
    Mp4,
}

impl ContainerFormat {
    pub fn ext(self) -> &'static str {
        match self {
            ContainerFormat::Mkv => "mkv",
            ContainerFormat::Mp4 => "mp4",
        }
    }
}

/// Download parameters passed from the UI.
#[derive(Clone, Debug)]
pub struct DownloadParams {
    pub url: String,
    pub output_dir: PathBuf,
    pub height: u32,
    pub mode: VideoMode,
    pub container: ContainerFormat,
    pub video_title: String,
}

/// Visible stage during download.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DownloadStage {
    Downloading,
    Cutting,
    Converting,
    Finalizing,
}

/// Events sent by worker threads to the UI thread.
/// The UI thread applies them with invoke_from_event_loop / timer draining.
#[derive(Debug)]
pub enum UiEvent {
    /// URL analysis completed.
    ProbeDone(crate::probe::VideoInfo),
    /// Analysis failure (self-contained: does not require output directory or log).
    ProbeError(String),
    /// Thumbnail bytes (decoded in UI with the `image` crate).
    ThumbnailReady(Result<Vec<u8>, String>),
    /// Download progress 0.0..=100.0.
    DownloadProgress { percent: f32 },
    /// Active stage changed.
    StageChanged(DownloadStage),
    /// A single line for the in-app live log.
    LogLine(String),
    /// Success: final file path.
    DownloadDone(String),
    /// User-friendly failure message (technical details are in download.log).
    DownloadError(String),
    /// Version check result (boot or manual).
    UpdateStatusReady { ytdlp: BinaryStatus, ffmpeg: BinaryStatus },
    /// Update progress (downloaded bytes -> approximate percentage).
    UpdateProgress { target: BinTarget, percent: f32, msg: String },
    /// Final summary: (target, success, detail/version).
    UpdateFinished { results: Vec<(BinTarget, bool, String)> },
}

# Downloader - yt-dlp + FFmpeg

Windows automation script and desktop application for downloading videos using **yt-dlp** and **FFmpeg**.

The application allows configuring:

* Destination folder for final files
* Video URL
* Output format: **MKV or MP4**
* Real available H.264 resolutions, listed by index and dimension
* Full video or specific clip
* Start and end timestamps for the clip
* Automatic option to open destination folder after download
* Fallback format selection when the preferred combination is unavailable
* Detailed log recording selected format IDs (`RESOLVED_FORMAT`)

The workflow is engineered to prioritize **download reliability and audio/video synchronization**, preserving original streams without re-encoding whenever possible.

---

## Requirements

Binaries reside in the `bin/` folder, which `download.bat` automatically prepends to `PATH`, giving them priority over system-wide installations:

* `bin\yt-dlp.exe`
* `bin\ffmpeg.exe` (the package also includes `ffplay.exe` and `ffprobe.exe`)

### Where to Download Binaries

If you prefer to download the binaries manually:

* **yt-dlp**: [GitHub Releases (Official)](https://github.com/yt-dlp/yt-dlp/releases/latest) — download `yt-dlp.exe`.
* **FFmpeg**: [BtbN FFmpeg-Builds](https://github.com/BtbN/FFmpeg-Builds/releases) or [Gyan.dev](https://www.gyan.dev/ffmpeg/builds/) (both recommended on ffmpeg.org) — download the Windows release and extract `ffmpeg.exe`, `ffprobe.exe`, and `ffplay.exe` into the `bin/` folder.

You can also use `update.bat` or the desktop GUI, which checks versions and downloads updates automatically into the `bin/` directory.

System-installed versions are only used as a fallback when present in `PATH`.

To verify `yt-dlp`:

```text
yt-dlp --version
```

To verify FFmpeg:

```text
ffmpeg -version
```

To verify binary resolution paths:

```text
where yt-dlp
where ffmpeg
```

The script also utilizes **PowerShell**, which is built into modern Windows versions.

---

## Project Structure

The repository structure is organized as follows:

```text
Downloader/
│
├── download.bat
├── update.bat
├── YoutubeDownloader.exe
├── bin/
│   ├── yt-dlp.exe
│   ├── ffmpeg.exe
│   ├── ffplay.exe
│   └── ffprobe.exe
├── app/
│   ├── Cargo.toml
│   └── src/
└── docs/
    ├── README.md
    ├── CHANGELOG.md
    ├── PLAN/
    └── sync.py
```

The `download.log` file is generated automatically inside the selected destination folder and does not need to exist beforehand.

---

## Graphical User Interface (YoutubeDownloader.exe)

`YoutubeDownloader.exe` is a native desktop application built with Rust and Slint that provides the functionality of both batch scripts in a modern dark-themed interface. It requires no installation or Python runtime: simply run the executable beside the `bin/` folder.

```text
YoutubeDownloader.exe
```

### Download Screen (Equivalent to download.bat)

* URL input field + **Analyze** button: fetches and displays title, duration, thumbnail preview, and available H.264 resolutions
* Format selection: **MKV or MP4**, content selection: **Full or Clip** (start/end timestamps in `HH:MM:SS` or `MM:SS`)
* Destination folder picker, real-time download progress bar, live log console, and action buttons: **Cancel**, **Open folder**, and **New**
* Generates the identical `download.log` file in the destination folder
* **View log** / **Hide log** button toggles the live log console in-place
* **Update binaries** button in the header navigates directly to the Update screen

### Update Screen (Equivalent to update.bat)

* Table displaying installed version, latest release version, and status badges for each binary
* Action buttons: **Update yt-dlp**, **Update FFmpeg**, and **Update all** — automatically disabled when all binaries are up-to-date
* **Delete binaries** button (red) with two-step confirmation to clean up the `bin/` directory
* **Check for updates** button with dynamic status feedback: "Checking..." → "Up to date" (green) or lists outdated binaries
* Color-coded feedback messages during and after operations (blue during check/update, green on success, red on error)
* Background version check on application startup with an update notification banner if new releases are detected
* Self-bootstrapping: if `bin/` is missing, the application automatically launches directly into this screen and downloads required tools
* **← Back** button returns to the Download screen

To compile from source code (`app/`) with the Rust toolchain installed:

```text
cd app
cargo build --release
copy target\release\YoutubeDownloader.exe ..\
```

---

## Updating Binaries

Run:

```text
update.bat
```

Before downloading anything, the script compares versions installed in `bin/` against the latest releases from official sources and displays installed version, latest release, and status (up-to-date or update available) for each tool:

* **yt-dlp** — `github.com/yt-dlp/yt-dlp` (fallback: official nightly builds repository)
* **FFmpeg** — `github.com/BtbN/FFmpeg-Builds` (fallback: `gyan.dev`, both recommended on ffmpeg.org)

After the version comparison, an interactive menu allows selecting what to update:

```text
1 - yt-dlp only
2 - FFmpeg only
3 - Both
0 - Exit without updating
```

You can also bypass the menu by passing the option argument directly on the command line:

```text
update.bat 3
```

Arguments `0`, `1`, `2`, and `3` map directly to the menu options. The script requires `curl` (included in Windows 10/11) and PowerShell.

---

## How to Run

Run:

```text
download.bat
```

The script displays a destination folder selection dialog and then prompts for download configuration.

### 1. Select Destination Folder

Choose the folder where the downloaded video and `download.log` will be saved. Temporary files remain isolated in `%TEMP%` and are cleaned up on completion.

### 2. Enter Video URL

Paste the video URL when prompted:

```text
Video URL: https://www.youtube.com/watch?v=xxxxxxxxxxx
```

### 3. Select Resolution

The script queries the H.264 streams actually present on YouTube and lists each by index, height, and dimensions (e.g. `4 - 480p - 854x480`). Enter the option number. The download prioritizes H.264 video at that resolution with Opus audio; if that combination is not available, it attempts fallbacks at the same resolution, a single progressive file, or the closest lower resolution. The format IDs are recorded in the log as `RESOLVED_FORMAT`.

### 4. Select Format

```text
1 - MKV
2 - MP4
```

### MKV

Preserves stream codecs without re-encoding. The preferred combination is:

```text
Video: H.264
Audio: Opus
```

This option provides maximum fidelity by preserving the original downloaded streams.

### MP4

Video is preserved without re-encoding as long as the selected codec is MP4-compatible:

```text
Video: selected codec, copied without re-encoding
```

Audio is converted to:

```text
Audio: AAC 320 kbps
```

This option provides maximum compatibility with players, mobile devices, and editing software that do not support Opus in MP4 containers.

---

### 5. Select Content

```text
1 - Full video
2 - Clip only
```

### Full Video

The complete video is downloaded and packaged into the selected format.

### Clip Only

The script prompts for timestamps:

```text
Clip start (HH:MM:SS or MM:SS):
Clip end (HH:MM:SS or MM:SS):
```

Example:

```text
Clip start: 00:03:01
Clip end: 00:04:50
```

Shorthand formats are also accepted:

```text
03:01
04:50
```

---

# Internal Pipeline

The script does not download isolated clip segments directly from YouTube servers.

To prevent audio/video desynchronization and connection failures during separated stream downloads, the workflow normalizes intermediate data to an MKV container via merging or remuxing without re-encoding, following this pipeline:

```text
                 YouTube
                    │
                    ▼
          yt-dlp downloads video
                    │
                    ▼
             Selected streams
        (preference: H.264 + Opus)
                    │
                    ▼
               source.mkv
                    │
           ┌────────┴────────┐
           │                 │
      Full video           Clip
           │                 │
           │          FFmpeg trims clip
           │                 │
           │             clip.mkv
           │                 │
           └────────┬────────┘
                    │
                    ▼
             Selected format
               /          \
             MKV          MP4
              │            │
              │     Compatible video copied
              │     Audio → AAC 320 kbps
              │            │
              └─────┬──────┘
                    ▼
                Final file
```

---

# Download Mechanism

`yt-dlp` is configured with the following stream preferences:

```text
Video: H.264 (avc1)
Audio: Opus
```

The underlying invocation includes parameters to ensure maximum download reliability:

```text
--force-ipv4
--retries 20
--fragment-retries 20
--retry-sleep 2
--print "RESOLVED_FORMAT: %(format_id)s"
--no-simulate
--merge-output-format mkv
--remux-video mkv
--verbose
```

### IPv4

`--force-ipv4` prevents routing issues with IPv6 network interfaces and YouTube CDN edge nodes.

### Retries

Transient network interruptions during segment downloads trigger automatic retries.

The resolved format ID is written to the log under `RESOLVED_FORMAT`. `--no-simulate` ensures the download executes when printing format information, and `--verbose` records diagnostic output.

---

# Video Trimming

When a clip is requested, the complete source video is downloaded first.

FFmpeg then performs lossless local trimming:

```text
source.mkv
     ↓
  FFmpeg
     ↓
 clip.mkv
```

Trimming uses:

```text
-c copy
```

Video and audio streams are copied directly at the container level without re-encoding, preserving exact original quality.

---

# MP4 Conversion

When MP4 output is selected:

```text
Video → copy
Audio → AAC 320 kbps
```

The H.264 video stream is copied without re-encoding. Only the Opus audio stream is converted to high-bitrate AAC, preventing secondary compression artifacts in the video track.

---

# Synchronization

The core design prevents synchronization drift between separate audio and video streams using a sequential pipeline:

```text
1. Download complete video stream
2. Download complete audio stream
3. Multiplex streams into source container
4. Perform local trimming if requested
5. Convert audio stream only when required
```

This strategy is significantly more reliable than directly fetching segmented HTTP byte ranges.

---

# Temporary Files

An isolated temporary directory is created inside `%TEMP%` for each run:

Example:

```text
C:\Users\User\AppData\Local\Temp\yt-dlp-video-123456789
```

Standard working contents:

```text
source.mkv
clip.mkv
```

On completion, this directory is cleaned up automatically. The final output is written directly to the user-selected destination directory.

---

# Output File Naming

File names are generated automatically from the sanitized video title:

### Clip

Example:

```text
VIDEO TITLE HERE - clip 00-03-01-00-04-50.mp4
```

### Full Video

Example:

```text
VIDEO TITLE HERE - full.mp4
```

Characters not permitted by the Windows file system are sanitized automatically via PowerShell to handle Unicode and accented characters reliably.

---

# Logging

Execution progress is logged automatically to:

```text
download.log
```

in the selected destination directory.

The log records:

* Execution start and end timestamps (`START:`, `FINISHED:`)
* Target URL
* Video title (`TITLE:`)
* Chosen resolution (`RESOLUTION:`)
* Output format (`FORMAT:`)
* Temporary working directory (`TEMP_DIR:`)
* Resolved yt-dlp format IDs (`RESOLVED_FORMAT:`)
* Clip boundaries when applicable (`Clip:`)
* Final output path (`FINAL:`)
* yt-dlp and FFmpeg diagnostic logs and error traces

If any stage fails, `download.log` provides diagnostic evidence to pinpoint the root cause.

---

# Output Stream Workflows

## MKV + Full Video

```text
YouTube
→ H.264 + Opus
→ source.mkv
→ final MKV file
```

## MKV + Clip

```text
YouTube
→ H.264 + Opus
→ source.mkv
→ FFmpeg trim
→ clip.mkv
```

## MP4 + Full Video

```text
YouTube
→ H.264 + Opus
→ source.mkv
→ H.264 stream copied
→ Opus converted to AAC 320 kbps
→ final MP4 file
```

## MP4 + Clip

```text
YouTube
→ H.264 + Opus
→ source.mkv
→ FFmpeg trim
→ H.264 stream copied
→ Opus converted to AAC 320 kbps
→ final MP4 file
```

---

# Format Selection Guide

### Choose MKV when:

* Maximum quality and fidelity are paramount
* You want to preserve the original Opus audio stream
* Your media player supports MKV containers
* Universal hardware compatibility is not required

### Choose MP4 when:

* Broad compatibility across devices is needed (mobile phones, TVs, web browsers, video editing suites)
* You require standard H.264 + AAC encoding
* Preserving the Opus audio codec is unnecessary

---

# Troubleshooting

## Script Does Not Start

Verify executable presence:

```text
bin\yt-dlp.exe --version
bin\ffmpeg.exe -version
```

If binaries are missing, run `update.bat` to download the latest releases into `bin/`. `download.bat` can also fall back to executables present in the system `PATH`.

---

## Unexpected FFmpeg Binary Used

Check binary lookup resolution:

```text
where ffmpeg
```

If multiple installations exist on your system, note that `download.bat` prioritizes binaries inside the local `bin/` directory over system `PATH`.

---

## Download Fails

Inspect:

```text
download.log
```

The log contains the complete verbose output from `yt-dlp`, indicating whether failures are due to network connectivity, YouTube rate limiting, geo-restrictions, or format availability.

---

## Filenames with Special Characters

The application uses PowerShell to sanitize invalid Windows file system characters while preserving Unicode titles, and configures the active console page to UTF-8 (`chcp 65001`).

---

# Technical Standards

The application follows these engineering priorities:

```text
1. Download reliability
2. Audio and video synchronization
3. Stream quality preservation
4. Output compatibility
```

Video streams are copied without re-encoding whenever the target container allows. Audio is re-encoded to AAC at 320 kbps only when target container compatibility requires it.

---

# License and Usage

This project is an automation utility leveraging `yt-dlp` and `FFmpeg`.

Users are responsible for ensuring that their downloads comply with:

* Applicable copyright laws and licenses
* Content creator permissions
* Local regulations
* Platform terms of service

Only use this tool to download media that you are authorized to access and store.

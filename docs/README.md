# Youtube Downloader

Windows automation for downloading videos with **yt-dlp** and **FFmpeg**, available both as batch scripts and as a desktop GUI.

The program lets you choose:

* Destination folder for the final files
* Video URL
* Output format: **MKV or MP4**
* Real H.264 resolutions available in the video, listed with dimensions
* Full video or just a clip
* Clip start and end timestamps
* Whether to open the destination folder after the download
* Fallback format selection when the preferred combination is unavailable
* Detailed log with the selected format IDs

The process prioritizes **download reliability and audio/video synchronization**, keeping streams unreencoded whenever possible.

---

## Requirements

The executables live in `bin/`, which `download.bat` automatically prepends to `PATH`, giving the bundled binaries priority over system installs:

* `bin\yt-dlp.exe`
* `bin\ffmpeg.exe` (the package also includes `ffplay.exe` and `ffprobe.exe`)

### Where to get the binaries

To download the binaries manually:

* **yt-dlp**: [GitHub Releases (Official)](https://github.com/yt-dlp/yt-dlp/releases/latest) — download `yt-dlp.exe`.
* **FFmpeg**: [BtbN FFmpeg-Builds](https://github.com/BtbN/FFmpeg-Builds/releases) or [Gyan.dev](https://www.gyan.dev/ffmpeg/builds/) (both recommended on ffmpeg.org) — download the Windows build and extract `ffmpeg.exe`, `ffprobe.exe`, and `ffplay.exe` into `bin/`.

You can also use `update.bat`, which compares versions and downloads updates automatically into `bin/`.

System-wide installs are only used as a fallback when present in `PATH`.

To check `yt-dlp`:

```text
yt-dlp --version
```

To check FFmpeg:

```text
ffmpeg -version
```

To check where the executables resolve from:

```text
where yt-dlp
where ffmpeg
```

The script also uses **PowerShell**, which is normally already available on Windows.

---

## Project files

Recommended layout:

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

`download.log` is created automatically in the selected destination folder and does not need to exist beforehand.

---

## Graphical interface (YoutubeDownloader.exe)

`YoutubeDownloader.exe` is a desktop app (Rust + Slint) that does exactly what both scripts do, with a dark visual interface. No install or Python required: just run it next to the `bin/` folder.

```text
YoutubeDownloader.exe
```

### Download screen (equivalent to download.bat)

* URL field + **Analyze** button: shows title, duration, thumbnail, and the real H.264 resolutions of the video
* **MKV or MP4** format, **full** video or **clip** (start/end in `HH:MM:SS` or `MM:SS`)
* Destination folder picker, real progress bar, live log, and **Cancel**, **Open folder**, and **New** buttons
* Writes the same `download.log` as the script into the destination folder
* **View log** toggles the in-app log panel (thumbnail and title stay visible; the panel swaps with the options card so everything fits on screen without scrolling)
* **Download** enables automatically once URL, resolution, and folder are set
* **Update binaries** button at the top opens the update screen

### Update screen (equivalent to update.bat)

* Table with installed version, latest version, and status badge per binary (**Up to date**, **Update available**, **Not installed**, **Unknown**)
* Buttons to update only yt-dlp, only FFmpeg, or both, with an OK/FAILED summary
* **Delete binaries** button (red) with two-step confirmation to clear `bin/`
* **Check for updates** shows live feedback ("Checking..." → "All tools are up to date" or lists outdated tools)
* Update buttons disable automatically when both binaries are already up to date
* On startup, versions are checked in the background and a badge appears when updates exist
* If `bin/` is missing, the app opens directly on this screen and installs everything itself
* **← Back** button returns to the download screen

To compile from source (`app/`) with the Rust toolchain installed:

```text
cd app
cargo build --release
copy target\release\YoutubeDownloader.exe ..\
```

---

## Updating the binaries

Run:

```text
update.bat
```

Before downloading anything, the script compares the versions installed in `bin/` against the latest official releases and shows, per binary, the installed version, the latest version, and the status (up to date or update available):

* **yt-dlp** — `github.com/yt-dlp/yt-dlp` (fallback: official nightly builds repository)
* **FFmpeg** — `github.com/BtbN/FFmpeg-Builds` (fallback: `gyan.dev`, both recommended on ffmpeg.org)

A menu then lets you choose what to update:

```text
1 - yt-dlp only
2 - FFmpeg only
3 - Both
0 - Exit without updating
```

You can also skip the menu by passing the option directly on the command line, which is useful for automation:

```text
update.bat 3
```

Options `0`, `1`, `2`, and `3` mean the same as in the menu. The script requires `curl` (included in Windows 10/11) and PowerShell.

---

## Running the script

Run:

```text
download.bat
```

The script opens a folder picker and then asks the download questions.

### 1. Choose the destination folder

Select the folder where the final file and `download.log` are written. Temporary files keep going to `%TEMP%` and are removed on completion.

### 2. Enter the link

Paste the video URL when asked:

```text
Link do video: https://www.youtube.com/watch?v=xxxxxxxxxxx
```

### 3. Choose the resolution

The script lists the H.264 resolutions that really exist in the video, each with a number, height, and dimensions (for example `4 - 480p - 854x480`). Type only the option number. The download prefers H.264 video at that resolution with Opus audio; when that combination is unavailable it tries other formats at the same resolution, a single progressive file, or the nearest resolution below. The codec may vary across these fallbacks. The log records the exact selected format IDs as `FORMATO_RESOLVIDO`.

### 4. Choose the format

```text
1 - MKV
2 - MP4
```

### MKV

Keeps the downloaded streams' codecs without re-encoding. The preferred combination is:

```text
Video: H.264
Audio: Opus
```

This best preserves the original codecs and therefore the original video and audio quality.

### MP4

Video stays unreencoded as long as the selected codec is MP4-compatible:

```text
Video: selected codec, copied without re-encoding
```

Audio is converted to:

```text
Audio: AAC 320 kbps
```

This offers wider compatibility with players, devices, and software that do not handle Opus in MP4 well.

---

### 5. Choose the content

```text
1 - Video completo
2 - Apenas um trecho
```

### Full video

The whole video is downloaded and converted to the chosen format.

### Clip

The script asks for:

```text
Inicio do trecho (HH:MM:SS ou MM:SS):
Final do trecho (HH:MM:SS ou MM:SS):
```

Example:

```text
Inicio do trecho: 00:03:01
Final do trecho: 00:04:50
```

Short form also works:

```text
03:01
04:50
```

---

# Internals

The script never downloads only the requested clip directly.

To reduce synchronization problems and failures while downloading separate streams, the script normalizes the intermediate file to MKV by merging or remuxing, without re-encoding the streams. The process is:

```text
                  YouTube
                     │
                     ▼
           yt-dlp downloads the video
                     │
                     ▼
           Selected streams
           (H.264 + Opus preferred)
                     │
                     ▼
               source.mkv
                     │
            ┌────────┴────────┐
            │                 │
       Full video          Clip
            │                 │
            │          FFmpeg cuts locally
            │                 │
            │           trecho.mkv
            │                 │
            └────────┬────────┘
                     │
                     ▼
              Chosen format
                /            \
              MKV             MP4
               │               │
                │       Compatible video is copied
                │       Audio → AAC 320 kbps
               │               │
               └───────┬───────┘
                       ▼
                 Final file
```

---

# Download

`yt-dlp` is instructed to prefer:

```text
Video: H.264 (avc1)
Audio: Opus
```

The script command also includes reliability mechanisms:

```text
--force-ipv4
--retries 20
--fragment-retries 20
--retry-sleep 2
--print "FORMATO_RESOLVIDO: %(format_id)s"
--no-simulate
--merge-output-format mkv
--remux-video mkv
--verbose
```

### IPv4

`--force-ipv4` avoids issues with certain IPv6 routes or connections to YouTube CDN servers.

### Retries

On temporary failures during download, `yt-dlp` retries.

The chosen format ID is recorded as `FORMATO_RESOLVIDO`. `--no-simulate` keeps the download running alongside that reporting option, and `--verbose` records details useful for diagnosing failures.

---

# Cutting the video

When a clip is requested, the full file is downloaded first.

Then FFmpeg cuts it locally:

```text
source.mkv
       ↓
FFmpeg
       ↓
trecho.mkv
```

Cutting uses:

```text
-c copy
```

So video and audio are not re-encoded during the cut.

Consequently, there is no additional quality loss at this step.

---

# MP4 conversion

When MP4 is selected, the script uses:

```text
Video → copy
Audio → AAC 320 kbps
```

H.264 video is not re-encoded.

Only Opus audio is converted to AAC.

This avoids a second compression generation on video while keeping good audio quality.

---

# Synchronization

The main goal is avoiding audio/video synchronization issues.

The script therefore follows this strategy:

```text
1. Download full video
2. Download full audio
3. Join the streams
4. Cut locally
5. Convert only when needed
```

This is more reliable than depending exclusively on direct downloads of small stream segments.

---

# Temporary files

During execution a temporary folder is created automatically inside `%TEMP%`.

Example:

```text
C:\Users\Usuario\AppData\Local\Temp\yt-dlp-video-123456789
```

It normally contains:

```text
source.mkv
trecho.mkv
```

After completion the folder is removed automatically. The final file stays in the selected destination folder.

---

# File names

The final file name is based automatically on the video title.

### Clip

Example:

```text
SAMU LINO DÁ SHOW E MENGÃO VENCE O CLÁSSICO COM GOLAÇOS FLAMENGO 3X0 BOTAFOGO - trecho 00-03-01-00-04-50.mp4
```

### Full video

Example:

```text
SAMU LINO DÁ SHOW E MENGÃO VENCE O CLÁSSICO COM GOLAÇOS FLAMENGO 3X0 BOTAFOGO - completo.mp4
```

Characters invalid in Windows file names are replaced automatically.

Name processing uses PowerShell to avoid issues with accented and special characters.

---

# Logs

The script automatically creates:

```text
download.log
```

in the selected destination folder.

The log records:

* Execution date and time
* URL used
* Video title
* Chosen format
* Download mode
* Clip timestamps
* Temporary folder
* Selected format IDs and full `yt-dlp` output
* FFmpeg output
* Errors during the process

After a successful download, the script asks whether to open the destination folder.

The file is especially useful for diagnosing failures.

When something goes wrong, `download.log` contents identify which step failed.

---

# Output flows

## MKV + full video

```text
YouTube
→ H.264 + Opus
→ source.mkv
→ final MKV file
```

## MKV + clip

```text
YouTube
→ H.264 + Opus
→ source.mkv
→ FFmpeg cut
→ trecho.mkv
```

## MP4 + full video

```text
YouTube
→ H.264 + Opus
→ source.mkv
→ H.264 untouched
→ Opus converted to AAC 320 kbps
→ MP4 file
```

## MP4 + clip

```text
YouTube
→ H.264 + Opus
→ source.mkv
→ FFmpeg cut
→ H.264 untouched
→ Opus converted to AAC 320 kbps
→ MP4 file
```

---

# Which format to choose?

### Use MKV when:

* original quality is the priority;
* you want to keep Opus;
* your player accepts MKV;
* maximum compatibility is unnecessary.

### Use MP4 when:

* compatibility is the priority;
* the file will play on phones, TVs, browsers, or software that prefers MP4;
* you want H.264 + AAC;
* preserving the original Opus audio is unnecessary.

---

# Troubleshooting

## The script does not start

Check:

```text
bin\yt-dlp.exe --version
```

and:

```text
bin\ffmpeg.exe -version
```

If the files are missing or a command is not recognized, run `update.bat` to download the latest binaries into `bin/`. `download.bat` can also use executables found in the system `PATH`.

---

## An unexpected FFmpeg is found

Check:

```text
where ffmpeg
```

Windows may find more than one installation.

Example:

```text
C:\Program Files\...\ffmpeg.exe
```

`download.bat` always prioritizes the executables in `bin/`; system installs are only considered when the folder is absent.

You can also check:

```text
ffmpeg -version
```

---

## The download fails

See:

```text
download.log
```

The log holds the full `yt-dlp` output and normally reveals whether the problem relates to the connection, YouTube, the selected format, or another component.

---

## The video name has odd characters

The script uses PowerShell to handle invalid Windows characters.

The script also starts with:

```text
chcp 65001
```

to work in UTF-8 and reduce issues with accented characters.

---

# Technical notes

The script prioritizes:

```text
1. Download reliability
2. Audio and video synchronization
3. Quality preservation
4. File compatibility
```

Selected video is copied without re-encoding when the chosen container supports it.

When MP4 is chosen, only audio is re-encoded to AAC 320 kbps.

When MKV is chosen, video and audio keep their original codecs whenever available.

---

# License and use

This project is only an automation tool using `yt-dlp` and `FFmpeg`.

You are responsible for using the tool in accordance with:

* applicable copyright;
* content permissions;
* local legislation;
* terms of service of the platforms used.

Only use the program for content you are authorized to download.

# Changelog

## [0.2.0] - 2026-09-27
-c Add update.bat updater

- New `update.bat` compares the installed versions against the latest official releases before downloading anything.
- Shows installed version, latest version, and up-to-date/outdated status for both yt-dlp and FFmpeg.
- Lets the user update only yt-dlp, only FFmpeg, or both; `update.bat 0|1|2|3` skips the menu for automation.
- Downloads yt-dlp from `github.com/yt-dlp/yt-dlp` (fallback: official nightly builds repo) and FFmpeg from `github.com/BtbN/FFmpeg-Builds` (fallback: `gyan.dev`), the builds recommended on ffmpeg.org.

-c Reorganize project layout

- Move the binaries to `bin/` and the documentation to `doc/`, leaving only the scripts at the root.
- `download.bat` now prepends `bin/` to the `PATH`, so the bundled executables take priority over system installs.

## [0.1.2] - 2026-09-25
-c Fix unavailable format downloads

- Fix cmd stripping `^` from the H.264 filter (`^^` escape), the root cause of "Requested format is not available" on every video.
- Fall back to same-height, progressive, or nearest-lower formats when H.264 plus Opus is unavailable.
- Show only resolutions that really exist in the video, with height and dimensions.
- Log the resolved yt-dlp format ids as FORMATO_RESOLVIDO.
-c Number resolution options

- Number each available H.264 resolution so users select it by position.

## [0.1.0] - 2026-09-24
-c Add destination and quality options

- Let users choose the output folder and H.264 resolution.
- Save the log beside the final video and optionally open the destination folder.
- Document the interactive workflow in `README.md`.

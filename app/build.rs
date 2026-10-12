fn main() {
    // Compile declarative UI.
    slint_build::compile("src/ui.slint").expect("failed to compile src/ui.slint");

    // Embed icon and metadata into .exe.
    let mut res = winresource::WindowsResource::new();
    res.set_icon("assets/icon.ico");
    res.set("ProductName", "Youtube Downloader");
    res.set("FileDescription", "Youtube Downloader - yt-dlp + FFmpeg (GUI)");
    res.set("OriginalFilename", "YoutubeDownloader.exe");
    res.compile().expect("failed to embed resources into exe");
}

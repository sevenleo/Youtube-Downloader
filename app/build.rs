fn main() {
    // Compila a UI declarativa.
    slint_build::compile("src/ui.slint").expect("falha ao compilar src/ui.slint");

    // Embute icone e metadados no .exe.
    let mut res = winresource::WindowsResource::new();
    res.set_icon("assets/icon.ico");
    res.set("ProductName", "Youtube Downloader");
    res.set("FileDescription", "Youtube Downloader - yt-dlp + FFmpeg (GUI)");
    res.set("OriginalFilename", "YoutubeDownloader.exe");
    res.compile().expect("falha ao embutir recursos no exe");
}

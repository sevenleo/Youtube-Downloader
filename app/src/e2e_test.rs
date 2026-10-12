//! Teste ponta a ponta HEADLESS do fluxo completo (sem GUI).
//! Roda apenas sob demanda: `cargo test -- --ignored`.
//! Baixa um video real (Big Buck Bunny, 144p), corta um trecho de 5s e gera MP4.

use std::path::PathBuf;
use std::sync::atomic::AtomicBool;
use std::sync::mpsc;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::state::{ContainerFormat, DownloadParams, UiEvent, VideoMode};

fn nanos() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos()
}

#[test]
#[ignore]
fn e2e_clip_144p_mp4() {
    let repo_bin = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("bin");
    let ytdlp = repo_bin.join("yt-dlp.exe");
    let ffmpeg = repo_bin.join("ffmpeg.exe");
    assert!(ytdlp.exists(), "bin/yt-dlp.exe nao encontrado");
    assert!(ffmpeg.exists(), "bin/ffmpeg.exe nao encontrado");

    let out = std::env::temp_dir().join(format!("yt-e2e-{}", nanos()));
    std::fs::create_dir_all(&out).unwrap();

    let (tx, rx) = mpsc::channel::<UiEvent>();
    // Drena eventos para o sender nunca bloquear (canal e unbounded, mas evita acumulo).
    std::thread::spawn(move || {
        let mut progresses = 0;
        while let Ok(ev) = rx.recv() {
            if matches!(ev, UiEvent::DownloadProgress { .. }) {
                progresses += 1;
            }
            let _ = progresses;
        }
    });

    let params = DownloadParams {
        url: "https://www.youtube.com/watch?v=aqz-KE-bpKQ".to_string(),
        output_dir: out.clone(),
        height: 144,
        mode: VideoMode::Clip {
            start: "00:00:05".to_string(),
            end: "00:00:10".to_string(),
        },
        container: ContainerFormat::Mp4,
        video_title: "Big Buck Bunny".to_string(),
    };
    let tools = crate::tools::ToolPaths { ytdlp, ffmpeg };
    let cancel = AtomicBool::new(false);

    let final_path =
        crate::download::run_download(&tools, &params, &tx, &cancel).expect("run_download falhou");
    assert!(final_path.exists(), "arquivo final nao existe");
    assert_eq!(
        final_path.extension().and_then(|e| e.to_str()),
        Some("mp4")
    );

    let log = std::fs::read_to_string(out.join("download.log")).expect("download.log ausente");
    for needle in [
        "INICIO:",
        "TITULO:",
        "RESOLUCAO: 144p",
        "FORMATO: mp4",
        "FORMATO_RESOLVIDO:",
        "FINALIZADO:",
    ] {
        assert!(log.contains(needle), "log sem {needle:?}");
    }

    std::fs::remove_dir_all(&out).ok();
}

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

//! Entry point da GUI. O `state.rs` e o contrato (congelado) entre UI e nucleo.
//! DONO DESTE ARQUIVO: agente de UI/wiring.

mod download;
mod log;
mod process;
mod probe;
mod state;
mod tools;
mod update;

#[cfg(test)]
mod e2e_test;

slint::include_modules!();

use std::path::PathBuf;
use std::rc::Rc;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    mpsc::{self, Sender},
    Arc, Mutex,
};

use slint::{Model, SharedString, VecModel};

use state::{
    BadgeState, BinTarget, ContainerFormat, DownloadParams, DownloadStage, UiEvent, VideoMode,
};

/// Segundos -> "HH:MM:SS".
fn fmt_duration(total_secs: u64) -> SharedString {
    format!(
        "{:02}:{:02}:{:02}",
        total_secs / 3600,
        (total_secs % 3600) / 60,
        total_secs % 60
    )
    .into()
}

/// Badge do contrato -> int da UI (0 ok, 1 update, 2 nao instalado, 3 desconhecido).
fn badge_to_int(b: BadgeState) -> i32 {
    match b {
        BadgeState::UpToDate => 0,
        BadgeState::UpdateAvailable => 1,
        BadgeState::NotInstalled => 2,
        BadgeState::Unknown => 3,
    }
}

fn stage_to_state(s: DownloadStage) -> SharedString {
    match s {
        DownloadStage::Downloading => "downloading",
        DownloadStage::Cutting => "cutting",
        DownloadStage::Converting => "converting",
        DownloadStage::Finalizing => "finalizing",
    }
    .into()
}

fn target_name(t: BinTarget) -> &'static str {
    match t {
        BinTarget::YtDlp => "yt-dlp",
        BinTarget::FFmpeg => "FFmpeg",
    }
}

/// "yt-dlp: OK (versao X)" / "FFmpeg: FALHOU (motivo)", um por linha.
fn fmt_update_summary(results: &[(BinTarget, bool, String)]) -> SharedString {
    results
        .iter()
        .map(|(t, ok, detail)| {
            if *ok {
                format!("{}: OK (versao {})", target_name(*t), detail)
            } else {
                format!("{}: FALHOU ({})", target_name(*t), detail)
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
        .into()
}

/// Acrescenta uma linha ao log ao vivo (chamado na thread da UI).
/// Limita o historico para nao degradar a renderizacao.
fn push_log(app: &AppWindow, line: String) {
    let model = app.get_log_lines();
    if model.row_count() > 1000 {
        for _ in 0..200 {
            let _ = model.remove_row(0);
        }
    }
    let _ = model.push_row(line.into());
}

/// Bytes da thumbnail -> `slint::Image` (falha = None, mantem placeholder).
fn decode_thumbnail(bytes: &[u8]) -> Option<slint::Image> {
    let rgb = image::load_from_memory(bytes).ok()?.to_rgb8();
    let (w, h) = (rgb.width(), rgb.height());
    if w == 0 || h == 0 {
        return None;
    }
    let mut buf = slint::SharedPixelBuffer::<slint::Rgb8Pixel>::new(w, h);
    buf.make_mut_bytes().copy_from_slice(&rgb.into_raw());
    Some(slint::Image::from_rgb8(buf))
}

fn status_text(v: &Option<String>) -> SharedString {
    v.clone().unwrap_or_else(|| "-".to_string()).into()
}

/// ensure_bin_dir -> read_installed -> http_client -> fetch_latest_versions
/// -> UpdateStatusReady. Nunca falha: sem dados = Unknown.
fn check_updates_and_emit(tx: &Sender<UiEvent>) {
    let (inst_y, inst_f) = match tools::ensure_bin_dir() {
        Ok(bin) => update::read_installed(&bin),
        Err(_) => (None, None),
    };
    let (lat_y, lat_f) = match update::http_client() {
        Ok(client) => update::fetch_latest_versions(&client),
        Err(_) => (None, None),
    };
    let _ = tx.send(UiEvent::UpdateStatusReady {
        ytdlp: update::make_status(inst_y, lat_y),
        ffmpeg: update::make_status(inst_f, lat_f),
    });
}

/// Aplica um UiEvent na janela. Roda SEMPRE na thread da UI
/// (via `upgrade_in_event_loop` no dispatcher).
fn apply_event(
    app: &AppWindow,
    ev: UiEvent,
    video: &Arc<Mutex<Option<probe::VideoInfo>>>,
) {
    match ev {
        UiEvent::ProbeDone(info) => {
            let res_list: Vec<SharedString> = info
                .resolutions
                .iter()
                .map(|r| format!("{}p ({}x{})", r.height, r.width, r.height).into())
                .collect();
            let n = res_list.len();
            *video.lock().unwrap() = Some(info.clone());
            app.set_video_title(info.title.into());
            app.set_duration(fmt_duration(info.duration_secs));
            app.set_resolutions(Rc::new(VecModel::from(res_list)).into());
            app.set_resolution_index(0);
            app.set_dl_state("ready".into());
            app.set_dl_percent(0.0);
            app.set_dl_error("".into());
            app.set_dl_final("".into());
            app.set_analysis_ok(true);
            push_log(app, format!("Análise concluída: {n} qualidades encontradas."));
        }
        UiEvent::ThumbnailReady(res) => {
            // Decodificado aqui (lado UI do dispatcher): `slint::Image` e !Send,
            // entao os bytes viajam pelo canal e a decodificacao ocorre na
            // thread da UI. Falha = mantem o placeholder.
            if let Ok(bytes) = res {
                if let Some(img) = decode_thumbnail(&bytes) {
                    app.set_thumbnail(img);
                }
            }
        }
        UiEvent::DownloadProgress { percent } => {
            app.set_dl_percent(percent.clamp(0.0, 100.0));
        }
        UiEvent::StageChanged(stage) => {
            app.set_dl_state(stage_to_state(stage));
        }
        UiEvent::LogLine(line) => {
            push_log(app, line);
        }
        UiEvent::DownloadDone(path) => {
            app.set_dl_state("done".into());
            app.set_dl_final(path.clone().into());
            app.set_dl_percent(100.0);
            app.set_analysis_ok(false);
            push_log(app, format!("Concluído: {path}"));
        }
        UiEvent::ProbeError(msg) => {
            app.set_dl_state("error".into());
            app.set_dl_error(msg.clone().into());
            app.set_analysis_ok(false);
            push_log(app, format!("ERRO: {msg}"));
        }
        UiEvent::DownloadError(msg) => {
            app.set_dl_state("error".into());
            app.set_dl_error(msg.clone().into());
            push_log(app, format!("ERRO: {msg}"));
        }
        UiEvent::UpdateStatusReady { ytdlp, ffmpeg } => {
            app.set_ytdlp_installed(status_text(&ytdlp.installed));
            app.set_ytdlp_latest(status_text(&ytdlp.latest));
            app.set_ytdlp_badge(badge_to_int(ytdlp.badge));
            app.set_ffmpeg_installed(status_text(&ffmpeg.installed));
            app.set_ffmpeg_latest(status_text(&ffmpeg.latest));
            app.set_ffmpeg_badge(badge_to_int(ffmpeg.badge));
        }
        UiEvent::UpdateProgress {
            target,
            percent,
            msg,
        } => {
            app.set_upd_busy(true);
            app.set_upd_percent(percent.clamp(0.0, 100.0));
            app.set_upd_msg(format!("{}: {msg}", target_name(target)).into());
        }
        UiEvent::UpdateFinished { results } => {
            app.set_upd_busy(false);
            app.set_upd_percent(100.0);
            app.set_upd_msg("Atualização concluída.".into());
            app.set_upd_summary(fmt_update_summary(&results));
        }
    }
}

fn main() -> anyhow::Result<()> {
    let app = AppWindow::new()?;
    app.set_log_lines(Rc::new(VecModel::from(Vec::<SharedString>::new())).into());
    app.set_resolutions(Rc::new(VecModel::from(Vec::<SharedString>::new())).into());

    // Estado Rust (nunca tocado direto por workers; so via canal).
    let (tx, rx) = mpsc::channel::<UiEvent>();
    let video: Arc<Mutex<Option<probe::VideoInfo>>> = Arc::new(Mutex::new(None));
    let cancel_slot: Arc<Mutex<Arc<AtomicBool>>> =
        Arc::new(Mutex::new(Arc::new(AtomicBool::new(false))));

    // ---- DISPATCHER: recebe UiEvents e aplica via event loop ----
    {
        let weak = app.as_weak();
        let video = video.clone();
        std::thread::spawn(move || {
            while let Ok(ev) = rx.recv() {
                let weak2 = weak.clone();
                let video2 = video.clone();
                let _ = weak2.upgrade_in_event_loop(move |app| apply_event(&app, ev, &video2));
            }
        });
    }

    // ---- on_analyze ----
    {
        let tx = tx.clone();
        let weak = app.as_weak();
        let video = video.clone();
        app.on_analyze(move || {
            let Some(ui) = weak.upgrade() else { return };
            if ui.get_dl_state() == "analyzing" {
                return; // ignora cliques repetidos durante a analise em curso
            }
            let url = ui.get_url().trim().to_string();
            if url.is_empty() {
                ui.set_dl_state("error".into());
                ui.set_dl_error("Informe a URL do vídeo.".into());
                return;
            }
            ui.set_dl_state("analyzing".into());
            ui.set_dl_error("".into());
            ui.set_dl_final("".into());
            ui.set_dl_percent(0.0);
            ui.set_analysis_ok(false);
            ui.set_video_title("Analisando…".into());
            ui.set_duration("".into());
            *video.lock().unwrap() = None;

            let txw = tx.clone();
            std::thread::spawn(move || {
                let tools = match tools::locate() {
                    Ok(t) => t,
                    Err(e) => {
                        let _ = txw.send(UiEvent::LogLine(format!(
                            "yt-dlp/ffmpeg não encontrados: {e:#}"
                        )));
                        let _ = txw.send(UiEvent::ProbeError(
                            "yt-dlp ou ffmpeg não encontrado. Abra a aba Atualizar para instalar."
                                .into(),
                        ));
                        return;
                    }
                };
                match probe::analyze(&tools.ytdlp, &url) {
                    Ok(info) => {
                        let thumb_url =
                            info.thumbnail_url.clone().filter(|u| !u.trim().is_empty());
                        let _ = txw.send(UiEvent::ProbeDone(info));
                        if let Some(tu) = thumb_url {
                            let bytes = update::http_client()
                                .and_then(|c| probe::fetch_thumbnail(&c, &tu));
                            let _ =
                                txw.send(UiEvent::ThumbnailReady(
                                    bytes.map_err(|e| e.to_string()),
                                ));
                        }
                    }
                    Err(e) => {
                        let _ = txw.send(UiEvent::LogLine(format!("Falha na análise: {e:#}")));
                        let _ = txw.send(UiEvent::ProbeError(format!(
                            "Não foi possível analisar o vídeo: {e:#}"
                        )));
                    }
                }
            });
        });
    }

    // ---- on_pick_folder ----
    {
        let weak = app.as_weak();
        app.on_pick_folder(move || {
            let Some(ui) = weak.upgrade() else { return };
            if let Some(dir) = rfd::FileDialog::new()
                .set_title("Escolher pasta de destino")
                .pick_folder()
            {
                ui.set_output_dir(dir.to_string_lossy().into_owned().into());
            }
        });
    }

    // ---- on_start_download ----
    {
        let tx = tx.clone();
        let weak = app.as_weak();
        let video = video.clone();
        let cancel_slot = cancel_slot.clone();
        app.on_start_download(move || {
            let Some(ui) = weak.upgrade() else { return };
            // Item 1: erro de validacao mostra so a mensagem (nao destroi `ready`);
            // o botao Baixar reage via binding can-download e o erro some via
            // handlers `changed` no .slint ao corrigir as opcoes.
            let fail = |ui: &AppWindow, msg: &str| {
                ui.set_dl_error(msg.into());
            };

            let url = ui.get_url().trim().to_string();
            if url.is_empty() {
                fail(&ui, "Informe a URL do vídeo.");
                return;
            }
            let info = match video.lock().unwrap().clone() {
                Some(i) => i,
                None => {
                    fail(&ui, "Clique em Analisar antes de baixar.");
                    return;
                }
            };
            let idx = ui.get_resolution_index() as usize;
            let Some(res) = info.resolutions.get(idx) else {
                fail(&ui, "Escolha uma resolução válida.");
                return;
            };
            let height = res.height;
            let out = ui.get_output_dir().trim().to_string();
            if out.is_empty() {
                fail(&ui, "Escolha a pasta de destino.");
                return;
            }
            let mode = if ui.get_mode() == 1 {
                let s = ui.get_start_t().trim().to_string();
                let e = ui.get_end_t().trim().to_string();
                if s.is_empty() || e.is_empty() {
                    fail(&ui, "Informe início e fim do trecho (HH:MM:SS).");
                    return;
                }
                VideoMode::Clip { start: s, end: e }
            } else {
                VideoMode::Full
            };
            let container = if ui.get_container() == 1 {
                ContainerFormat::Mp4
            } else {
                ContainerFormat::Mkv
            };
            let params = DownloadParams {
                url,
                output_dir: PathBuf::from(out),
                height,
                mode,
                container,
                video_title: info.title.clone(),
            };

            // Cancel novo para este download.
            let cancel = Arc::new(AtomicBool::new(false));
            *cancel_slot.lock().unwrap() = cancel.clone();

            ui.set_dl_state("downloading".into());
            ui.set_dl_error("".into());
            ui.set_dl_final("".into());
            ui.set_dl_percent(0.0);

            let txw = tx.clone();
            std::thread::spawn(move || {
                let tools = match tools::locate() {
                    Ok(t) => t,
                    Err(e) => {
                        let _ = txw.send(UiEvent::LogLine(format!(
                            "yt-dlp/ffmpeg não encontrados: {e:#}"
                        )));
                        let _ = txw.send(UiEvent::DownloadError(
                            "yt-dlp ou ffmpeg não encontrado. Abra a aba Atualizar para instalar."
                                .into(),
                        ));
                        return;
                    }
                };
                match download::run_download(&tools, &params, &txw, &cancel) {
                    Ok(path) => {
                        let _ = txw.send(UiEvent::DownloadDone(
                            path.to_string_lossy().into_owned(),
                        ));
                    }
                    Err(e) => {
                        let _ = txw.send(UiEvent::LogLine(format!("Falha no download: {e:#}")));
                        if cancel.load(Ordering::SeqCst) {
                            let _ = txw
                                .send(UiEvent::DownloadError("Download cancelado.".into()));
                        } else {
                            let _ = txw.send(UiEvent::DownloadError(format!(
                                "Falha no download: {e:#}"
                            )));
                        }
                    }
                }
            });
        });
    }

    // ---- on_cancel_download ----
    {
        let tx = tx.clone();
        let cancel_slot = cancel_slot.clone();
        app.on_cancel_download(move || {
            cancel_slot.lock().unwrap().store(true, Ordering::SeqCst);
            let _ = tx.send(UiEvent::LogLine("Cancelamento solicitado…".into()));
        });
    }

    // ---- on_open_folder ----
    {
        let weak = app.as_weak();
        app.on_open_folder(move || {
            let Some(ui) = weak.upgrade() else { return };
            let dir = ui.get_output_dir().trim().to_string();
            if dir.is_empty() {
                return;
            }
            let _ = std::process::Command::new("explorer").arg(&dir).spawn();
        });
    }

    // ---- on_new_download: reseta para idle ----
    {
        let weak = app.as_weak();
        let video = video.clone();
        app.on_new_download(move || {
            let Some(ui) = weak.upgrade() else { return };
            *video.lock().unwrap() = None;
            ui.set_dl_state("idle".into());
            ui.set_dl_percent(0.0);
            ui.set_dl_error("".into());
            ui.set_dl_final("".into());
            ui.set_analysis_ok(false);
            ui.set_show_log(false);
            ui.set_video_title("".into());
            ui.set_duration("".into());
            ui.set_resolutions(Rc::new(VecModel::from(Vec::<SharedString>::new())).into());
            ui.set_resolution_index(0);
            ui.set_log_lines(Rc::new(VecModel::from(Vec::<SharedString>::new())).into());
        });
    }

    // ---- on_view_log: toggle do painel de log in-app (item 2) ----
    {
        let weak = app.as_weak();
        app.on_view_log(move || {
            let Some(ui) = weak.upgrade() else { return };
            ui.set_show_log(!ui.get_show_log());
        });
    }

    // ---- on_check_updates (manual) ----
    {
        let tx = tx.clone();
        let weak = app.as_weak();
        app.on_check_updates(move || {
            if let Some(ui) = weak.upgrade() {
                ui.set_upd_msg("Verificando versões…".into());
            }
            let txw = tx.clone();
            std::thread::spawn(move || check_updates_and_emit(&txw));
        });
    }

    // ---- on_run_update(mask: 1 yt-dlp, 2 ffmpeg, 3 ambos) ----
    {
        let tx = tx.clone();
        let weak = app.as_weak();
        app.on_run_update(move |mask| {
            let targets: Vec<BinTarget> = match mask {
                1 => vec![BinTarget::YtDlp],
                2 => vec![BinTarget::FFmpeg],
                _ => vec![BinTarget::YtDlp, BinTarget::FFmpeg],
            };
            if let Some(ui) = weak.upgrade() {
                ui.set_upd_busy(true);
                ui.set_upd_percent(0.0);
                ui.set_upd_msg("Iniciando atualização…".into());
                ui.set_upd_summary("".into());
            }
            let txw = tx.clone();
            std::thread::spawn(move || {
                let results = match tools::ensure_bin_dir() {
                    Ok(bin) => match update::run_update(&bin, &targets, &txw) {
                        Ok(r) => r,
                        Err(e) => targets
                            .iter()
                            .map(|t| (*t, false, format!("{e:#}")))
                            .collect(),
                    },
                    Err(e) => targets
                        .iter()
                        .map(|t| (*t, false, format!("pasta bin: {e:#}")))
                        .collect(),
                };
                let _ = txw.send(UiEvent::UpdateFinished { results });
                // Re-check de status apos o update.
                check_updates_and_emit(&txw);
            });
        });
    }

    // ---- BOOT: checagem automatica de versoes em background ----
    {
        app.set_upd_msg("Verificando versões em segundo plano…".into());
        let txb = tx.clone();
        std::thread::spawn(move || check_updates_and_emit(&txb));
    }

    // ---- BOOT (item 5): sem binarios, abre direto na tela de atualizar ----
    if tools::locate().is_err() {
        app.set_screen(1);
        app.set_missing_tools_msg(
            "yt-dlp ou ffmpeg não encontrado. Instale os binários abaixo para começar a baixar."
                .into(),
        );
    }

    app.run()?;
    Ok(())
}

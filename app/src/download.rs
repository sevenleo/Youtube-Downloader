//! Fluxo de download equivalente ao download.bat. Ver plano: paridade download.bat.
//! DONO DESTE ARQUIVO: agente core-download.

use anyhow::Result;
use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicBool;
use std::sync::mpsc::Sender;

use crate::log::DownloadLog;
use crate::process::{run_streamed, ProcLine};
use crate::state::{DownloadParams, DownloadStage, UiEvent, VideoMode};
use crate::tools::ToolPaths;

/// Titulo -> nome de arquivo seguro (caracteres invalidos do Windows -> `_`).
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

/// Constroi a format string exata do .bat para a altura `height`.
/// Ex.: `bv*[height=1080][vcodec^=avc1]+ba[acodec=opus]/bv*[height=1080]+ba/...`
///
/// NOTA: sem os escapes do cmd (`%%` -> `%`, `^^` -> `^`); em Rust os
/// argumentos vao literalmente para o processo (sem shell).
pub fn format_string(height: u32) -> String {
    format!(
        "bv*[height={h}][vcodec^=avc1]+ba[acodec=opus]/bv*[height={h}]+ba/b[height={h}]/bv*[height<={h}]+ba/best[height<={h}]",
        h = height
    )
}

/// Guarda RAII da pasta temporaria: remove `%TEMP%\yt-dlp-video-*` no Drop,
/// ou seja, limpa mesmo em erro/cancelamento (equivale aos `rmdir` dos ERROR_*).
struct TempGuard {
    path: PathBuf,
}

impl Drop for TempGuard {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.path);
    }
}

/// Registra uma linha do processo no log E na UI; extrai `%` de download.
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

/// Erro amigavel estilo ERROR_* do .bat; o detalhe tecnico vai ao log.
fn fail(dlog: &mut DownloadLog, friendly: &str, detail: impl std::fmt::Display) -> anyhow::Error {
    let _ = dlog.line(&format!("{friendly} Detalhe: {detail}"));
    anyhow::anyhow!("{friendly}")
}

/// Executa o fluxo completo (download -> corte opcional -> mkv/mp4 -> arquivo final),
/// emitindo `UiEvent`s de progresso/etapa/log. Cancela via `cancel` (mata o filho e
/// limpa temporarios - use guarda RAII). Retorna o caminho do arquivo final.
/// Erros amigaveis equivalentes aos ERROR_* do .bat (detalhe tecnico vai ao log).
pub fn run_download(
    tools: &ToolPaths,
    params: &DownloadParams,
    emit: &Sender<UiEvent>,
    cancel: &AtomicBool,
) -> Result<PathBuf> {
    use std::sync::atomic::Ordering;
    use std::time::{SystemTime, UNIX_EPOCH};

    let cancelled = || cancel.load(Ordering::SeqCst);

    // download.log recriado do zero (truncate) a cada download, como o .bat.
    // Falha aqui = pasta invalida/sem permissao (ERROR_DESTINATION).
    let mut dlog = DownloadLog::create(&params.output_dir)
        .map_err(|e| anyhow::anyhow!("ERRO: selecione uma pasta de destino valida e gravavel. ({e})"))?;

    // Checagem inicial (ERROR_YTDLP / ERROR_FFMPEG).
    if !tools.ytdlp.is_file() {
        let msg = "ERRO: yt-dlp nao foi encontrado na pasta bin nem no PATH.";
        let _ = dlog.line(msg);
        anyhow::bail!("{msg}");
    }
    if !tools.ffmpeg.is_file() {
        let msg = "ERRO: ffmpeg nao foi encontrado na pasta bin nem no PATH.";
        let _ = dlog.line(msg);
        anyhow::bail!("{msg}");
    }

    // Metadados no log, como o .bat (URL/TITULO/RESOLUCAO/FORMATO/INICIO/FINAL).
    let _ = dlog.line(&format!("URL: {}", params.url));
    let _ = dlog.line(&format!("TITULO: {}", params.video_title));
    let _ = dlog.line(&format!("RESOLUCAO: {}p", params.height));
    let _ = dlog.line(&format!("FORMATO: {}", params.container.ext()));
    if let VideoMode::Clip { start, end } = &params.mode {
        if start.trim().is_empty() {
            let msg = "ERRO: o inicio do trecho nao pode ficar vazio.";
            let _ = dlog.line(msg);
            anyhow::bail!("{msg}");
        }
        if end.trim().is_empty() {
            let msg = "ERRO: o final do trecho nao pode ficar vazio.";
            let _ = dlog.line(msg);
            anyhow::bail!("{msg}");
        }
        let _ = dlog.line(&format!("INICIO: {start}"));
        let _ = dlog.line(&format!("FINAL: {end}"));
    }

    // Pasta temporaria %TEMP%\yt-dlp-video-<nanos> (ERROR_TEMP).
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let temp_dir = std::env::temp_dir().join(format!("yt-dlp-video-{nanos}"));
    std::fs::create_dir_all(&temp_dir).map_err(|e| {
        fail(
            &mut dlog,
            "ERRO: nao foi possivel criar a pasta temporaria.",
            e,
        )
    })?;
    let _temp_guard = TempGuard {
        path: temp_dir.clone(),
    };
    let _ = dlog.line(&format!("PASTA TEMPORARIA: {}", temp_dir.display()));

    let progress_re = regex::Regex::new(r"\[download\]\s+(\d+(?:\.\d+)?)%")
        .map_err(|e| fail(&mut dlog, "ERRO: falha interna (regex).", e))?;

    // --- yt-dlp (mesmos comandos do .bat + `--newline` p/ progresso por linha) ---
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
        "FORMATO_RESOLVIDO: %(format_id)s",
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

    // A linha `FORMATO_RESOLVIDO: X` (do --print) passa por handle_line como
    // todas as outras: vai ao log E a UI. Aqui so detectamos presenca p/ diagnostico.
    let mut resolved_format: Option<String> = None;
    let dl_code = run_streamed(
        &tools.ytdlp,
        &ytdlp_args,
        &mut |line: ProcLine| {
            if let Some(id) = line.text.strip_prefix("FORMATO_RESOLVIDO:") {
                resolved_format = Some(id.trim().to_string());
            }
            handle_line(&mut dlog, emit, &progress_re, line);
        },
        cancel,
    )
    .map_err(|e| fail(&mut dlog, "ERRO: falha no download.", format!("yt-dlp: {e}")))?;
    if cancelled() {
        let msg = "Download cancelado pelo usuario.";
        let _ = dlog.line(msg);
        anyhow::bail!("{msg}");
    }
    if dl_code != 0 {
        return Err(fail(
            &mut dlog,
            "ERRO: falha no download. Consulte o download.log para detalhes.",
            format!("exit code {dl_code}"),
        ));
    }
    if resolved_format.is_none() {
        let _ = dlog.line("Aviso: formato resolvido nao detectado na saida do yt-dlp.");
    }

    // Exige source.mkv (ERROR_SOURCE).
    let source = temp_dir.join("source.mkv");
    if !source.is_file() {
        return Err(fail(
            &mut dlog,
            "ERRO: source.mkv nao foi encontrado.",
            "source.mkv inexistente apos o yt-dlp",
        ));
    }
    let _ = dlog.line("DOWNLOAD CONCLUIDO");

    // --- Corte (so no modo trecho; ERROR_CUT) ---
    let mut input = source.clone();
    if let VideoMode::Clip { start, end } = &params.mode {
        let _ = emit.send(UiEvent::StageChanged(DownloadStage::Cutting));
        let trecho = temp_dir.join("trecho.mkv");
        let _ = dlog.line(&format!("Corte: {start} ate {end}"));
        let source_s = source.to_string_lossy().into_owned();
        let trecho_s = trecho.to_string_lossy().into_owned();
        let cut_args: Vec<String> = vec![
            "-y", "-ss", start, "-to", end, "-i", &source_s, "-map", "0", "-c", "copy",
            &trecho_s,
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
        .map_err(|e| fail(&mut dlog, "ERRO: falha ao cortar o trecho.", format!("ffmpeg: {e}")))?;
        if cancelled() {
            let msg = "Download cancelado pelo usuario.";
            let _ = dlog.line(msg);
            anyhow::bail!("{msg}");
        }
        if cut_code != 0 || !trecho.is_file() {
            return Err(fail(
                &mut dlog,
                "ERRO: falha ao cortar o trecho.",
                format!("exit code {cut_code}"),
            ));
        }
        input = trecho;
    }

    // --- Saida final ---
    let final_name = final_file_name(params);
    let final_path = params.output_dir.join(&final_name);
    let _ = dlog.line(&format!("FINAL: {}", final_path.display()));

    match params.container {
        crate::state::ContainerFormat::Mkv => {
            // MKV = copia direta do intermediario (copy /b do .bat).
            std::fs::copy(&input, &final_path).map_err(|e| {
                fail(
                    &mut dlog,
                    "ERRO: arquivo final nao foi criado.",
                    format!("copia: {e}"),
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
                    "ERRO: falha na conversao para MP4.",
                    format!("ffmpeg: {e}"),
                )
            })?;
            if cancelled() {
                let msg = "Download cancelado pelo usuario.";
                let _ = dlog.line(msg);
                anyhow::bail!("{msg}");
            }
            if conv_code != 0 {
                return Err(fail(
                    &mut dlog,
                    "ERRO: falha na conversao para MP4.",
                    format!("exit code {conv_code}"),
                ));
            }
        }
    }

    let _ = emit.send(UiEvent::StageChanged(DownloadStage::Finalizing));
    // Antes de declarar sucesso, verifica que o arquivo final existe (ERROR_FINAL).
    if !final_path.is_file() {
        return Err(fail(
            &mut dlog,
            "ERRO: arquivo final nao foi criado.",
            "arquivo final inexistente",
        ));
    }

    // Limpa temporarios (o Drop da guarda garante em todos os caminhos).
    let _ = std::fs::remove_dir_all(&temp_dir);
    dlog.finish()?;
    Ok(final_path)
}

/// Constroi o nome do arquivo final: `TITULO - completo.ext` ou
/// `TITULO - trecho HH-MM-SS-HH-MM-SS.ext` (dois-pontos viram hifen).
pub fn final_file_name(params: &DownloadParams) -> String {
    let safe = sanitize_filename(&params.video_title);
    let ext = params.container.ext();
    match &params.mode {
        VideoMode::Full => format!("{safe} - completo.{ext}"),
        VideoMode::Clip { start, end } => {
            let s = start.replace(':', "-");
            let e = end.replace(':', "-");
            format!("{safe} - trecho {s}-{e}.{ext}")
        }
    }
}

/// Apenas o path do download.log para a pasta de destino (sem criar).
pub fn log_path(output_dir: &Path) -> PathBuf {
    output_dir.join("download.log")
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
        assert_eq!(sanitize_filename("  ola mundo  "), "ola mundo");
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
            sanitize_filename("Big Buck Bunny (4K) - Teste!"),
            "Big Buck Bunny (4K) - Teste!"
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
        let p = params("Meu Video", ContainerFormat::Mp4, VideoMode::Full);
        assert_eq!(final_file_name(&p), "Meu Video - completo.mp4");
        let p = params("Meu Video", ContainerFormat::Mkv, VideoMode::Full);
        assert_eq!(final_file_name(&p), "Meu Video - completo.mkv");
    }

    #[test]
    fn final_name_clip_colons_become_hyphens() {
        let p = params(
            "Meu Video",
            ContainerFormat::Mkv,
            VideoMode::Clip {
                start: "00:01:02".to_string(),
                end: "03:04".to_string(),
            },
        );
        assert_eq!(final_file_name(&p), "Meu Video - trecho 00-01-02-03-04.mkv");
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
            "A_B_C - trecho 00-00-10-00-00-20.mp4"
        );
    }
}

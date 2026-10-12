//! Analise da URL via `yt-dlp --dump-single-json`. Ver plano: paridade download.bat.
//! DONO DESTE ARQUIVO: agente core-download.

use anyhow::{Context, Result};
use std::path::Path;

use crate::process::run_capture_stdout;

/// Uma resolucao H.264 realmente disponivel no video.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct H264Resolution {
    pub height: u32,
    /// Maior largura entre os formatos dessa altura.
    pub width: u32,
}

/// Metadados do video necessarios para a tela e para o download.
#[derive(Clone, Debug)]
pub struct VideoInfo {
    pub title: String,
    /// Duracao em segundos (0 se desconhecida).
    pub duration_secs: u64,
    /// URL da miniatura (campo `thumbnail` do JSON), se houver.
    pub thumbnail_url: Option<String>,
    /// Resolucoes H.264 (vcodec iniciado por "avc1"), ordenadas por altura.
    pub resolutions: Vec<H264Resolution>,
}

impl VideoInfo {
    /// Titulo seguro para nome de arquivo (chars invalidos do Windows viram `_`).
    pub fn safe_title(&self) -> String {
        crate::download::sanitize_filename(&self.title)
    }
}

/// Numero > 0 vindo do JSON (yt-dlp emite ints; aceita float/string por robustez,
/// como o cast `[int]` do .bat).
fn json_u32_positive(v: &serde_json::Value) -> Option<u32> {
    if let Some(n) = v.as_u64() {
        return u32::try_from(n).ok().filter(|&x| x > 0);
    }
    if let Some(n) = v.as_i64() {
        return if n > 0 { u32::try_from(n).ok() } else { None };
    }
    if let Some(n) = v.as_f64() {
        return if n.is_finite() && n > 0.0 {
            Some(n as u32).filter(|&x| x > 0)
        } else {
            None
        };
    }
    if let Some(s) = v.as_str() {
        if let Ok(n) = s.trim().parse::<u32>() {
            return if n > 0 { Some(n) } else { None };
        }
    }
    None
}

/// Roda `yt-dlp --force-ipv4 --dump-single-json --skip-download --no-warnings <url>`
/// e extrai titulo/duracao/thumbnail/resolucoes H.264. Erros mapeados como no .bat
/// (titulo vazio = ERROR_TITLE; sem resolucao H.264 = ERROR_FORMATS).
pub fn analyze(ytdlp: &Path, url: &str) -> Result<VideoInfo> {
    if url.trim().is_empty() {
        anyhow::bail!("ERRO: nenhum link foi informado.");
    }
    let args: Vec<String> = [
        "--force-ipv4",
        "--dump-single-json",
        "--skip-download",
        "--no-warnings",
        url,
    ]
    .iter()
    .map(|s| s.to_string())
    .collect();

    let out = run_capture_stdout(ytdlp, &args)
        .context("falha ao executar yt-dlp --dump-single-json")?;
    let v: serde_json::Value = serde_json::from_str(&out)
        .map_err(|_| anyhow::anyhow!("ERRO: nao foi possivel obter o titulo."))?;

    let title = v
        .get("title")
        .and_then(|t| t.as_str())
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .ok_or_else(|| anyhow::anyhow!("ERRO: nao foi possivel obter o titulo."))?;

    // Formatos com vcodec comecando com "avc1" e height/width > 0,
    // agrupados por altura (maior largura do grupo), ordenados por altura.
    let mut by_height: std::collections::BTreeMap<u32, u32> = std::collections::BTreeMap::new();
    if let Some(formats) = v.get("formats").and_then(|f| f.as_array()) {
        for f in formats {
            let is_h264 = f
                .get("vcodec")
                .and_then(|c| c.as_str())
                .map(|c| c.starts_with("avc1"))
                .unwrap_or(false);
            if !is_h264 {
                continue;
            }
            let height = f.get("height").and_then(json_u32_positive);
            let width = f.get("width").and_then(json_u32_positive);
            if let (Some(h), Some(w)) = (height, width) {
                by_height
                    .entry(h)
                    .and_modify(|max_w| *max_w = (*max_w).max(w))
                    .or_insert(w);
            }
        }
    }
    let resolutions: Vec<H264Resolution> = by_height
        .into_iter()
        .map(|(height, width)| H264Resolution { height, width })
        .collect();
    if resolutions.is_empty() {
        anyhow::bail!("ERRO: nao foram encontradas resolucoes H.264 para este video.");
    }

    let duration_secs = v
        .get("duration")
        .map(|d| {
            if let Some(n) = d.as_u64() {
                n
            } else if let Some(n) = d.as_i64() {
                n.max(0) as u64
            } else if let Some(n) = d.as_f64() {
                if n.is_finite() && n > 0.0 {
                    n as u64
                } else {
                    0
                }
            } else {
                0
            }
        })
        .unwrap_or(0);

    // Campo `thumbnail`; fallback: ultima entrada de `thumbnails` com url.
    let thumbnail_url = v
        .get("thumbnail")
        .and_then(|t| t.as_str())
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())
        .or_else(|| {
            v.get("thumbnails")
                .and_then(|t| t.as_array())
                .and_then(|arr| {
                    arr.iter()
                        .rev()
                        .filter_map(|t| t.get("url").and_then(|u| u.as_str()))
                        .map(str::trim)
                        .find(|s| !s.is_empty())
                        .map(|s| s.to_string())
                })
        });

    Ok(VideoInfo {
        title: title.to_string(),
        duration_secs,
        thumbnail_url,
        resolutions,
    })
}

/// Baixa a miniatura (para a UI exibir). Falha nao quebra o fluxo.
pub fn fetch_thumbnail(client: &reqwest::blocking::Client, url: &str) -> Result<Vec<u8>> {
    let bytes = client
        .get(url)
        .send()
        .context("falha ao baixar miniatura")?
        .error_for_status()
        .context("miniatura retornou status de erro")?
        .bytes()
        .context("falha ao ler bytes da miniatura")?;
    Ok(bytes.to_vec())
}

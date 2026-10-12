//! Localizacao de yt-dlp/ffmpeg. Ver plano: "Resolucao de caminhos".
//! DONO DESTE ARQUIVO: agente core-download.

use anyhow::{Context, Result};
use std::path::{Path, PathBuf};

/// Caminhos resolvidos dos binarios usados pela aba Download.
#[derive(Clone, Debug)]
pub struct ToolPaths {
    pub ytdlp: PathBuf,
    pub ffmpeg: PathBuf,
}

fn exe_dir() -> Option<PathBuf> {
    std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|d| d.to_path_buf()))
}

/// Procura um binario (primeiro nome que existir vence) na ordem:
/// 1. `bin/` ao lado do exe em execucao (caso normal);
/// 2. `bin/` subindo ate 5 niveis a partir do exe (p/ `cargo run` -> target/debug);
/// 3. `PATH` do sistema (fallback dos .bat).
fn find_binary(names: &[&str]) -> Option<PathBuf> {
    // (1) + (2): nivel 0 = dir do exe; sobe ate 5 niveis.
    if let Some(mut dir) = exe_dir() {
        for _ in 0..=5 {
            for name in names {
                let candidate = dir.join("bin").join(name);
                if candidate.is_file() {
                    return Some(candidate);
                }
            }
            match dir.parent() {
                Some(parent) => dir = parent.to_path_buf(),
                None => break,
            }
        }
    }
    // (3): PATH (com e sem extensao .exe).
    if let Ok(path_var) = std::env::var("PATH") {
        for entry in std::env::split_paths(&path_var) {
            // Remove aspas que alguns instaladores deixam nas entradas.
            let entry_str = entry.to_string_lossy();
            let trimmed = entry_str.trim_matches('"');
            let entry = Path::new(trimmed);
            for name in names {
                let candidate = entry.join(name);
                if candidate.is_file() {
                    return Some(candidate);
                }
            }
        }
    }
    None
}

/// Localiza yt-dlp.exe e ffmpeg.exe. Ordem (a primeira que servir vence):
/// 1. `bin/` ao lado do exe em execucao (caso normal);
/// 2. `bin/` na raiz do repositorio (subindo dirs a partir de current_exe, p/ `cargo run`);
/// 3. `PATH` do sistema (fallback dos .bat).
pub fn locate() -> Result<ToolPaths> {
    let ytdlp = find_binary(&["yt-dlp.exe", "yt-dlp"])
        .ok_or_else(|| anyhow::anyhow!("ERRO: yt-dlp nao foi encontrado na pasta bin nem no PATH."))?;
    let ffmpeg = find_binary(&["ffmpeg.exe", "ffmpeg"])
        .ok_or_else(|| anyhow::anyhow!("ERRO: ffmpeg nao foi encontrado na pasta bin nem no PATH."))?;
    Ok(ToolPaths { ytdlp, ffmpeg })
}

/// Pasta `bin` canonica do projeto (ao lado do exe); criada se nao existir.
/// Usada pela aba Atualizar, inclusive quando os binarios ainda nao existem.
pub fn ensure_bin_dir() -> Result<PathBuf> {
    // (1): sobe niveis do exe procurando pasta existente que contenha `bin/`.
    if let Some(mut dir) = exe_dir() {
        for _ in 0..=5 {
            let bin = dir.join("bin");
            if bin.is_dir() {
                // Garante existencia (cria se preciso) e retorna.
                std::fs::create_dir_all(&bin)
                    .with_context(|| format!("nao foi possivel criar {}", bin.display()))?;
                return Ok(bin);
            }
            match dir.parent() {
                Some(parent) => dir = parent.to_path_buf(),
                None => break,
            }
        }
    }
    // (2): `<dir-do-exe>/bin` + create_dir_all.
    let fallback = exe_dir()
        .map(|d| d.join("bin"))
        .unwrap_or_else(|| PathBuf::from("bin"));
    std::fs::create_dir_all(&fallback)
        .with_context(|| format!("nao foi possivel criar {}", fallback.display()))?;
    Ok(fallback)
}

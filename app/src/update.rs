//! Atualizador equivalente ao update.bat. Ver plano: paridade update.bat.
//! DONO DESTE ARQUIVO: agente core-update.

use anyhow::{Context, Result};
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::sync::mpsc::Sender;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use crate::state::{BadgeState, BinTarget, BinaryStatus, UiEvent};

/// Fontes do yt-dlp: oficial primeiro, fallback nightly (mesma ordem do .bat).
const YTDLP_SOURCES: [&str; 2] = [
    "https://github.com/yt-dlp/yt-dlp/releases/latest/download/yt-dlp.exe",
    "https://github.com/yt-dlp/yt-dlp-nightly-builds/releases/latest/download/yt-dlp.exe",
];

/// Fontes do FFmpeg: oficial BtbN primeiro, fallback gyan.dev (mesma ordem do .bat).
const FFMPEG_SOURCES: [&str; 2] = [
    "https://github.com/BtbN/FFmpeg-Builds/releases/latest/download/ffmpeg-master-latest-win64-gpl.zip",
    "https://www.gyan.dev/ffmpeg/builds/ffmpeg-release-essentials.zip",
];

/// Tentativas por fonte (equivale ao `curl --retry 3` do .bat).
const ATTEMPTS_PER_SOURCE: u32 = 3;

/// Passo aproximado de emissao de progresso (em pontos percentuais).
const PROGRESS_STEP: f32 = 5.0;

/// Cliente HTTP para a aba Atualizar e a thumbnail.
/// Reqwest blocking + rustls + `User-Agent: yt-updater` (obrigatorio p/ API do GitHub).
pub fn http_client() -> Result<reqwest::blocking::Client> {
    reqwest::blocking::Client::builder()
        .user_agent("yt-updater")
        .timeout(Duration::from_secs(30))
        .build()
        .context("falha ao criar cliente HTTP")
}

/// Consulta versoes mais recentes: (tag yt-dlp, data-yyyyMMdd do build FFmpeg).
/// Cada uma vira None se falhar ("indisponivel") - nunca propaga erro.
pub fn fetch_latest_versions(
    client: &reqwest::blocking::Client,
) -> (Option<String>, Option<String>) {
    (fetch_ytdlp_tag(client), fetch_ffmpeg_build_date(client))
}

fn fetch_ytdlp_tag(client: &reqwest::blocking::Client) -> Option<String> {
    let v: serde_json::Value = client
        .get("https://api.github.com/repos/yt-dlp/yt-dlp/releases/latest")
        .send()
        .ok()?
        .error_for_status()
        .ok()?
        .json()
        .ok()?;
    let tag = v.get("tag_name")?.as_str()?.trim();
    if tag.is_empty() {
        None
    } else {
        Some(tag.to_string())
    }
}

fn fetch_ffmpeg_build_date(client: &reqwest::blocking::Client) -> Option<String> {
    let v: serde_json::Value = client
        .get("https://api.github.com/repos/BtbN/FFmpeg-Builds/releases/latest")
        .send()
        .ok()?
        .error_for_status()
        .ok()?
        .json()
        .ok()?;
    let published = v.get("published_at")?.as_str()?;
    published_to_yyyymmdd(published.trim())
}

/// `published_at` ja vem em UTC ("2026-10-05T12:34:56Z"): yyyyMMdd = ano+mes+dia
/// por fatiamento da string, sem conversao de fuso (igual ao .bat).
fn published_to_yyyymmdd(s: &str) -> Option<String> {
    let b = s.as_bytes();
    if b.len() < 10 || b[4] != b'-' || b[7] != b'-' {
        return None;
    }
    let (y, m, d) = (&s[0..4], &s[5..7], &s[8..10]);
    if !y.bytes().all(|c| c.is_ascii_digit())
        || !m.bytes().all(|c| c.is_ascii_digit())
        || !d.bytes().all(|c| c.is_ascii_digit())
    {
        return None;
    }
    Some(format!("{y}{m}{d}"))
}

/// Le versoes instaladas em `bin_dir`: (yt-dlp, ffmpeg-builddate).
/// yt-dlp: 1a linha de `--version`. ffmpeg: regex `-(\d{8})` na 1a linha de `-version`
/// (se nao casar, linha inteira). Ausente = None ("nao instalado").
pub fn read_installed(bin_dir: &Path) -> (Option<String>, Option<String>) {
    let ytdlp = first_line_of(&bin_dir.join("yt-dlp.exe"), "--version");

    let ffmpeg_line = first_line_of(&bin_dir.join("ffmpeg.exe"), "-version");
    let ffmpeg = ffmpeg_line.map(|line| {
        match regex::Regex::new(r"-(\d{8})") {
            Ok(re) => re
                .captures(&line)
                .and_then(|c| c.get(1))
                .map(|m| m.as_str().to_string())
                .unwrap_or(line),
            Err(_) => line,
        }
    });

    (ytdlp, ffmpeg)
}

/// Roda `program arg` e devolve a 1a linha aparada do stdout.
/// Qualquer falha (ausente, erro de spawn, saida vazia) vira None.
fn first_line_of(program: &Path, arg: &str) -> Option<String> {
    let out = crate::process::run_capture_stdout(program, &[arg.to_string()]).ok()?;
    let line = out.lines().next().unwrap_or("").trim();
    if line.is_empty() {
        None
    } else {
        Some(line.to_string())
    }
}

/// Regras EXATAS do update.bat (comparacao case-insensitive).
pub fn badge_of(installed: Option<&str>, latest: Option<&str>) -> BadgeState {
    match (installed, latest) {
        (None, Some(_)) => BadgeState::NotInstalled,
        (Some(_), None) => BadgeState::Unknown,
        (None, None) => BadgeState::Unknown,
        (Some(a), Some(b)) => {
            if a.eq_ignore_ascii_case(b) {
                BadgeState::UpToDate
            } else {
                BadgeState::UpdateAvailable
            }
        }
    }
}

/// Guarda RAII: a pasta temporaria e removida no drop (sucesso OU falha),
/// como o `rmdir /s /q "%TEMP_DIR%"` do .bat.
struct TempGuard(PathBuf);

impl Drop for TempGuard {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

/// Cria `%TEMP%\yt-updater-<nanos>-<pid>` (unicidade sem crate rand).
fn unique_temp_dir() -> Result<PathBuf> {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let dir =
        std::env::temp_dir().join(format!("yt-updater-{nanos}-{}", std::process::id()));
    fs::create_dir_all(&dir)
        .with_context(|| format!("nao foi possivel criar a pasta {}", dir.display()))?;
    Ok(dir)
}

/// Writer que envolve o arquivo de destino e emite `UpdateProgress` durante o
/// `copy_to` (streaming: nunca carrega tudo na RAM). Percentual via
/// content-length; -1.0 quando o tamanho e desconhecido.
struct ProgressWriter<'a> {
    inner: fs::File,
    emit: &'a Sender<UiEvent>,
    target: BinTarget,
    label: &'static str,
    total: Option<u64>,
    written: u64,
    last_sent: f32,
}

impl<'a> ProgressWriter<'a> {
    fn percent(&self) -> f32 {
        match self.total {
            Some(t) if t > 0 => ((self.written as f32 / t as f32) * 100.0).min(100.0),
            _ => -1.0,
        }
    }

    fn send(&mut self, percent: f32) {
        self.last_sent = percent;
        let msg = if percent < 0.0 {
            format!("Baixando {}...", self.label)
        } else {
            format!("Baixando {}... {percent:.0}%", self.label)
        };
        let _ = self.emit.send(UiEvent::UpdateProgress {
            target: self.target,
            percent,
            msg,
        });
    }

    fn emit_progress(&mut self, force: bool) {
        let percent = self.percent();
        if force {
            self.send(percent);
        } else if percent < 0.0 {
            // Tamanho desconhecido: so inicio/fim (evita spam de eventos).
        } else if percent >= 100.0 || percent - self.last_sent >= PROGRESS_STEP {
            self.send(percent);
        }
    }
}

impl Write for ProgressWriter<'_> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let n = self.inner.write(buf)?;
        self.written += n as u64;
        self.emit_progress(false);
        Ok(n)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.inner.flush()
    }
}

/// Baixa UMA url em streaming (`response.copy_to`, nunca tudo na RAM).
fn download_one(
    client: &reqwest::blocking::Client,
    url: &str,
    dest: &Path,
    target: BinTarget,
    label: &'static str,
    emit: &Sender<UiEvent>,
) -> Result<()> {
    let mut resp = client
        .get(url)
        .send()
        .with_context(|| format!("falha de rede ao baixar {label} de {url}"))?;
    let status = resp.status();
    if !status.is_success() {
        anyhow::bail!("HTTP {status} ao baixar {label} de {url}");
    }
    let total = resp.content_length();
    let file = fs::File::create(dest)
        .with_context(|| format!("nao foi possivel criar {}", dest.display()))?;
    let mut out = ProgressWriter {
        inner: file,
        emit,
        target,
        label,
        total,
        written: 0,
        last_sent: 0.0,
    };
    out.emit_progress(true); // 0%
    resp.copy_to(&mut out)
        .with_context(|| format!("falha ao gravar {label}"))?;
    out.emit_progress(true); // 100% (ou -1.0 se tamanho desconhecido)
    Ok(())
}

/// Baixa com RETRY 3 por fonte e fallback para a proxima fonte (como o .bat:
/// oficial primeiro, so depois da ultima falha tenta a alternativa).
fn download_with_fallback(
    client: &reqwest::blocking::Client,
    sources: &[&str],
    dest: &Path,
    target: BinTarget,
    label: &'static str,
    emit: &Sender<UiEvent>,
    missing_msg: &str,
) -> Result<()> {
    let mut last_err = String::from("sem detalhes");
    for url in sources {
        for attempt in 1..=ATTEMPTS_PER_SOURCE {
            match download_one(client, url, dest, target, label, emit) {
                Ok(()) => return Ok(()),
                Err(e) => {
                    last_err = format!("{e:#}");
                    let _ = emit.send(UiEvent::UpdateProgress {
                        target,
                        percent: -1.0,
                        msg: format!(
                            "{label}: tentativa {attempt}/{ATTEMPTS_PER_SOURCE} em {url} falhou; tentando novamente..."
                        ),
                    });
                }
            }
        }
    }
    Err(anyhow::anyhow!("{missing_msg} ({last_err})"))
}

/// Atualiza o yt-dlp: baixa (oficial -> fallback nightly), copia por cima de
/// `bin\yt-dlp.exe` e reexecuta `--version` para o detalhe (como o .bat).
fn update_ytdlp(
    client: &reqwest::blocking::Client,
    bin_dir: &Path,
    temp_dir: &Path,
    emit: &Sender<UiEvent>,
) -> Result<String> {
    let staged = temp_dir.join("yt-dlp.exe");
    download_with_fallback(
        client,
        &YTDLP_SOURCES,
        &staged,
        BinTarget::YtDlp,
        "yt-dlp",
        emit,
        "nao foi possivel baixar o yt-dlp de nenhuma fonte",
    )?;
    let dest = bin_dir.join("yt-dlp.exe");
    fs::copy(&staged, &dest)
        .with_context(|| "nao foi possivel copiar yt-dlp.exe para a pasta bin")?;
    let detail = first_line_of(&dest, "--version").unwrap_or_else(|| "yt-dlp atualizado".to_string());
    let _ = emit.send(UiEvent::UpdateProgress {
        target: BinTarget::YtDlp,
        percent: 100.0,
        msg: format!("yt-dlp atualizado: {detail}"),
    });
    Ok(detail)
}

/// Extrai o zip do FFmpeg para `bin_dir`: localiza `*/bin/ffmpeg.exe`
/// (case-insensitive, barras `/`), extrai ffmpeg/ffplay/ffprobe do MESMO
/// diretorio bin SEMPRE via `enclosed_name()` (anti zip-slip).
fn extract_ffmpeg_zip(zip_path: &Path, bin_dir: &Path) -> Result<()> {
    let file =
        fs::File::open(zip_path).context("nao foi possivel abrir o pacote do FFmpeg")?;
    let mut zip =
        zip::ZipArchive::new(file).context("nao foi possivel extrair o pacote do FFmpeg")?;

    // (indice, nome normalizado) de todas as entradas.
    let mut entries: Vec<(usize, String)> = Vec::new();
    for i in 0..zip.len() {
        if let Some(name) = zip.name_for_index(i) {
            entries.push((i, name.replace('\\', "/").to_lowercase()));
        }
    }

    let ffmpeg_entry = entries
        .iter()
        .find(|(_, n)| n.ends_with("/bin/ffmpeg.exe") || *n == "bin/ffmpeg.exe")
        .map(|(i, n)| (*i, n.clone()));
    let ffmpeg_norm = ffmpeg_entry
        .map(|(_, n)| n)
        .ok_or_else(|| anyhow::anyhow!("ffmpeg.exe nao foi encontrado no pacote"))?;
    // Prefixo do diretorio bin dentro do pacote (ex.: "x/bin/").
    let bin_prefix = ffmpeg_norm
        .strip_suffix("ffmpeg.exe")
        .unwrap_or("")
        .to_string();

    // ffmpeg.exe e obrigatorio; ffplay/ffprobe sao extraidos se presentes no
    // mesmo bin (ex.: o fallback gyan essentials nao traz ffplay.exe).
    for exe in ["ffmpeg.exe", "ffplay.exe", "ffprobe.exe"] {
        let wanted = format!("{bin_prefix}{exe}");
        let idx = entries.iter().find(|(_, n)| *n == wanted).map(|(i, _)| *i);
        let Some(i) = idx else {
            if exe == "ffmpeg.exe" {
                anyhow::bail!("ffmpeg.exe nao foi encontrado no pacote");
            }
            continue;
        };
        let mut entry = zip
            .by_index(i)
            .with_context(|| format!("nao foi possivel ler {exe} no pacote"))?;
        let safe = entry
            .enclosed_name()
            .ok_or_else(|| anyhow::anyhow!("entrada insegura no pacote: {wanted}"))?;
        let fname = safe
            .file_name()
            .ok_or_else(|| anyhow::anyhow!("entrada invalida no pacote: {wanted}"))?;
        let dest = bin_dir.join(fname);
        let mut out = fs::File::create(&dest)
            .with_context(|| format!("nao foi possivel copiar {}", dest.display()))?;
        io::copy(&mut entry, &mut out)
            .with_context(|| format!("nao foi possivel copiar os binarios do FFmpeg"))?;
    }
    Ok(())
}

/// Atualiza o FFmpeg: baixa o zip (BtbN -> fallback gyan), extrai os 3 exes
/// para `bin\` e mostra a 1a linha de `ffmpeg -version` (como o .bat).
fn update_ffmpeg(
    client: &reqwest::blocking::Client,
    bin_dir: &Path,
    temp_dir: &Path,
    emit: &Sender<UiEvent>,
) -> Result<String> {
    let zip_path = temp_dir.join("ffmpeg.zip");
    download_with_fallback(
        client,
        &FFMPEG_SOURCES,
        &zip_path,
        BinTarget::FFmpeg,
        "FFmpeg",
        emit,
        "nao foi possivel baixar o FFmpeg de nenhuma fonte",
    )?;
    let _ = emit.send(UiEvent::UpdateProgress {
        target: BinTarget::FFmpeg,
        percent: 100.0,
        msg: "Extraindo pacote do FFmpeg...".to_string(),
    });
    extract_ffmpeg_zip(&zip_path, bin_dir)?;
    let detail = first_line_of(&bin_dir.join("ffmpeg.exe"), "-version")
        .unwrap_or_else(|| "FFmpeg atualizado".to_string());
    let _ = emit.send(UiEvent::UpdateProgress {
        target: BinTarget::FFmpeg,
        percent: 100.0,
        msg: format!("FFmpeg atualizado: {detail}"),
    });
    Ok(detail)
}

/// Executa as atualizacoes pedidas (com fallbacks e resumo), emitindo UiEvent de
/// progresso; retorna (alvo, sucesso, detalhe) por binario. Ver plano para URLs e
/// sequencias exatas (retry 3 por fonte, zip via `enclosed_name()`, copia os 3 exes).
///
/// Falha num alvo NAO aborta o outro (resumo `false` + motivo). A pasta
/// temporaria e limpa SEMPRE (guarda RAII), em sucesso ou falha.
pub fn run_update(
    bin_dir: &Path,
    targets: &[BinTarget],
    emit: &Sender<UiEvent>,
) -> Result<Vec<(BinTarget, bool, String)>> {
    let client = http_client()?;
    let temp_dir = unique_temp_dir()?;
    let _guard = TempGuard(temp_dir.clone());
    // Equivale ao `if not exist ... mkdir` do .bat (ERROR_BIN).
    fs::create_dir_all(bin_dir)
        .with_context(|| format!("nao foi possivel criar a pasta {}", bin_dir.display()))?;

    let mut results = Vec::new();
    if targets.contains(&BinTarget::YtDlp) {
        match update_ytdlp(&client, bin_dir, &temp_dir, emit) {
            Ok(detail) => results.push((BinTarget::YtDlp, true, detail)),
            Err(e) => results.push((BinTarget::YtDlp, false, format!("{e:#}"))),
        }
    }
    if targets.contains(&BinTarget::FFmpeg) {
        match update_ffmpeg(&client, bin_dir, &temp_dir, emit) {
            Ok(detail) => results.push((BinTarget::FFmpeg, true, detail)),
            Err(e) => results.push((BinTarget::FFmpeg, false, format!("{e:#}"))),
        }
    }
    Ok(results)
    // `_guard` sai de escopo aqui e remove a pasta temporaria SEMPRE.
}

/// Monta o status completo de um binario a partir de instalada/mais recente.
pub fn make_status(installed: Option<String>, latest: Option<String>) -> BinaryStatus {
    BinaryStatus {
        badge: badge_of(installed.as_deref(), latest.as_deref()),
        installed,
        latest,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn badge_equal_is_up_to_date() {
        assert_eq!(
            badge_of(Some("2026.01.01"), Some("2026.01.01")),
            BadgeState::UpToDate
        );
    }

    #[test]
    fn badge_equal_ignores_ascii_case() {
        assert_eq!(badge_of(Some("ABC"), Some("abc")), BadgeState::UpToDate);
    }

    #[test]
    fn badge_different_is_update_available() {
        assert_eq!(
            badge_of(Some("2025.01.01"), Some("2026.01.01")),
            BadgeState::UpdateAvailable
        );
    }

    #[test]
    fn badge_not_installed() {
        assert_eq!(
            badge_of(None, Some("2026.01.01")),
            BadgeState::NotInstalled
        );
    }

    #[test]
    fn badge_latest_unavailable_is_unknown() {
        assert_eq!(
            badge_of(Some("2026.01.01"), None),
            BadgeState::Unknown
        );
    }

    #[test]
    fn badge_both_missing_is_unknown() {
        assert_eq!(badge_of(None, None), BadgeState::Unknown);
    }

    #[test]
    fn published_at_to_yyyymmdd() {
        assert_eq!(
            published_to_yyyymmdd("2026-10-05T12:34:56Z"),
            Some("20261005".to_string())
        );
        assert_eq!(published_to_yyyymmdd(""), None);
        assert_eq!(published_to_yyyymmdd("curta"), None);
        assert_eq!(published_to_yyyymmdd("2026/10/05"), None);
    }
}

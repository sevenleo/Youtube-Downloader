//! CONTRATO CONGELADO entre UI e nucleo da aplicacao.
//! NENHUM agente pode alterar este arquivo. Se algo estiver errado ou faltar,
//! resolva dentro dos SEUS arquivos e reporte na resposta final.

use std::path::PathBuf;

/// Binario gerenciado pela aba Atualizar.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BinTarget {
    YtDlp,
    FFmpeg,
}

/// Status de um binario (badge na UI). Regras de inferencia no plano (update.bat).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BadgeState {
    /// Verde: instalado == mais recente.
    UpToDate,
    /// Amarelo: instalado != mais recente.
    UpdateAvailable,
    /// Vermelho: nao instalado e ha versao disponivel.
    NotInstalled,
    /// Cinza: nao foi possivel verificar.
    Unknown,
}

/// Versao instalada + mais recente + badge, para um binario.
#[derive(Clone, Debug)]
pub struct BinaryStatus {
    pub installed: Option<String>,
    pub latest: Option<String>,
    pub badge: BadgeState,
}

/// Conteudo baixado: inteiro ou trecho com timestamps ("HH:MM:SS" ou "MM:SS").
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum VideoMode {
    Full,
    Clip { start: String, end: String },
}

/// Conteiner de saida.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ContainerFormat {
    Mkv,
    Mp4,
}

impl ContainerFormat {
    pub fn ext(self) -> &'static str {
        match self {
            ContainerFormat::Mkv => "mkv",
            ContainerFormat::Mp4 => "mp4",
        }
    }
}

/// Parametros de um download, vindos da UI.
#[derive(Clone, Debug)]
pub struct DownloadParams {
    pub url: String,
    pub output_dir: PathBuf,
    pub height: u32,
    pub mode: VideoMode,
    pub container: ContainerFormat,
    pub video_title: String,
}

/// Etapa visivel durante o download.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DownloadStage {
    Downloading,
    Cutting,
    Converting,
    Finalizing,
}

/// Eventos enviados por threads de trabalho para a thread de UI.
/// A thread de UI aplica com invoke_from_event_loop / drenagem por timer.
#[derive(Debug)]
pub enum UiEvent {
    /// Analise da URL concluida.
    ProbeDone(crate::probe::VideoInfo),
    /// Falha na analise (semantica propria: nao exige pasta/log definidos).
    ProbeError(String),
    /// Bytes da miniatura (decodificar na UI com a crate `image`).
    ThumbnailReady(Result<Vec<u8>, String>),
    /// Progresso de download 0.0..=100.0.
    DownloadProgress { percent: f32 },
    /// Mudou a etapa visivel.
    StageChanged(DownloadStage),
    /// Uma linha para o log ao vivo da UI.
    LogLine(String),
    /// Sucesso: caminho do arquivo final.
    DownloadDone(String),
    /// Falha amigavel (detalhe tecnico ja foi ao download.log).
    DownloadError(String),
    /// Resultado da checagem de versoes (boot ou manual).
    UpdateStatusReady { ytdlp: BinaryStatus, ffmpeg: BinaryStatus },
    /// Progresso de um update (bytes baixados -> percentual aproximado).
    UpdateProgress { target: BinTarget, percent: f32, msg: String },
    /// Resumo final: (alvo, sucesso, detalhe/versao).
    UpdateFinished { results: Vec<(BinTarget, bool, String)> },
}

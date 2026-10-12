//! Escrita do download.log (mesmo conteudo do .bat). Ver plano.
//! DONO DESTE ARQUIVO: agente core-download.

use anyhow::{Context, Result};
use chrono::Local;
use std::fs::OpenOptions;
use std::io::Write;
use std::path::{Path, PathBuf};

const DIVIDER: &str = "==================================================";

fn now_br() -> String {
    Local::now().format("%d/%m/%Y %H:%M:%S").to_string()
}

/// Log de um download. Criado (truncate) na pasta de destino.
pub struct DownloadLog {
    path: PathBuf,
}

impl DownloadLog {
    /// Cria/trunca `download.log` na pasta de destino e escreve o cabecalho
    /// ("===...=", "INICIO: dd/mm/aaaa hh:mm:ss"). Falha = pasta invalida/sem permissao.
    pub fn create(output_dir: &Path) -> Result<Self> {
        let path = output_dir.join("download.log");
        let mut f = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(&path)
            .with_context(|| format!("nao foi possivel criar {}", path.display()))?;
        writeln!(f, "{DIVIDER}")?;
        writeln!(f, "INICIO: {}", now_br())?;
        writeln!(f, "{DIVIDER}")?;
        f.flush()?;
        Ok(Self { path })
    }

    /// Acrescenta uma linha (UTF-8 sem BOM).
    pub fn line(&mut self, text: &str) -> Result<()> {
        let mut f = OpenOptions::new()
            .write(true)
            .create(true)
            .append(true)
            .open(&self.path)
            .with_context(|| format!("nao foi possivel escrever em {}", self.path.display()))?;
        writeln!(f, "{text}")?;
        f.flush()?;
        Ok(())
    }

    /// Linha final "FINALIZADO: dd/mm/aaaa hh:mm:ss" + divisor; fecha o log.
    pub fn finish(&mut self) -> Result<()> {
        let mut f = OpenOptions::new()
            .write(true)
            .create(true)
            .append(true)
            .open(&self.path)
            .with_context(|| format!("nao foi possivel finalizar {}", self.path.display()))?;
        writeln!(f, "FINALIZADO: {}", now_br())?;
        writeln!(f, "{DIVIDER}")?;
        f.flush()?;
        Ok(())
    }

    pub fn path(&self) -> &Path {
        &self.path
    }
}

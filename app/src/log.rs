//! Writing download.log (same content as the .bat script).
//! Core download logging module.

use anyhow::{Context, Result};
use chrono::Local;
use std::fs::OpenOptions;
use std::io::Write;
use std::path::{Path, PathBuf};

const DIVIDER: &str = "==================================================";

fn now_timestamp() -> String {
    Local::now().format("%d/%m/%Y %H:%M:%S").to_string()
}

/// Download log tracker. Created (truncated) in the destination folder.
pub struct DownloadLog {
    path: PathBuf,
}

impl DownloadLog {
    /// Creates/truncates `download.log` in destination folder and writes the header
    /// ("===...=", "START: dd/mm/yyyy hh:mm:ss"). Failure = invalid/unwritable folder.
    pub fn create(output_dir: &Path) -> Result<Self> {
        let path = output_dir.join("download.log");
        let mut f = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(&path)
            .with_context(|| format!("could not create {}", path.display()))?;
        writeln!(f, "{DIVIDER}")?;
        writeln!(f, "START: {}", now_timestamp())?;
        writeln!(f, "{DIVIDER}")?;
        f.flush()?;
        Ok(Self { path })
    }

    /// Appends a line (UTF-8 without BOM).
    pub fn line(&mut self, text: &str) -> Result<()> {
        let mut f = OpenOptions::new()
            .write(true)
            .create(true)
            .append(true)
            .open(&self.path)
            .with_context(|| format!("could not write to {}", self.path.display()))?;
        writeln!(f, "{text}")?;
        f.flush()?;
        Ok(())
    }

    /// Final line "FINISHED: dd/mm/yyyy hh:mm:ss" + divider; closes the log.
    pub fn finish(&mut self) -> Result<()> {
        let mut f = OpenOptions::new()
            .write(true)
            .create(true)
            .append(true)
            .open(&self.path)
            .with_context(|| format!("could not finalize {}", self.path.display()))?;
        writeln!(f, "FINISHED: {}", now_timestamp())?;
        writeln!(f, "{DIVIDER}")?;
        f.flush()?;
        Ok(())
    }

    pub fn path(&self) -> &Path {
        &self.path
    }
}

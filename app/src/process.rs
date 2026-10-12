//! Spawn de processos SEM janela de console e captura de saida.
//! DONO DESTE ARQUIVO: agente core-download. Ver plano: process.rs.

use std::io;
use std::path::Path;
use std::sync::atomic::AtomicBool;

/// Origem de uma linha capturada.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProcStream {
    Stdout,
    Stderr,
}

/// Uma linha de saida do processo (ja decodificada com UTF-8 lossy).
#[derive(Clone, Debug)]
pub struct ProcLine {
    pub stream: ProcStream,
    pub text: String,
}

/// Le um pipe ate EOF separando por `\n` (NAO quebra por `\r`, pois o yt-dlp
/// e chamado com `--newline`) e envia cada linha decodificada com UTF-8 lossy.
fn spawn_reader(
    pipe: impl std::io::Read + Send + 'static,
    stream: ProcStream,
    tx: std::sync::mpsc::Sender<ProcLine>,
) -> std::thread::JoinHandle<()> {
    std::thread::spawn(move || {
        let mut reader = std::io::BufReader::new(pipe);
        let mut buf: Vec<u8> = Vec::new();
        loop {
            buf.clear();
            let n = {
                use std::io::BufRead;
                match reader.read_until(b'\n', &mut buf) {
                    Ok(n) => n,
                    Err(_) => break,
                }
            };
            if n == 0 {
                break; // EOF
            }
            if buf.ends_with(b"\n") {
                buf.pop();
            }
            if buf.ends_with(b"\r") {
                buf.pop();
            }
            let text = String::from_utf8_lossy(&buf).into_owned();
            if tx.send(ProcLine { stream, text }).is_err() {
                break; // receptor foi embora
            }
        }
    })
}

/// Executa `program` com `args` (SEM shell, SEM console) consumindo stdout+stderr
/// linha a linha e chamando `sink` para cada uma (mesma ordem temporal aproximada;
/// equivale ao `2>&1` dos .bat). Se `cancel` virar true, mata o processo.
/// Retorna o codigo de saida. Nao quebra linhas por `\r` (usar `--newline` no yt-dlp).
pub fn run_streamed(
    program: &Path,
    args: &[String],
    sink: &mut dyn FnMut(ProcLine),
    cancel: &AtomicBool,
) -> io::Result<i32> {
    use std::os::windows::process::CommandExt;
    use std::process::{Command, Stdio};
    use std::sync::atomic::Ordering;
    use std::sync::mpsc;
    use std::time::Duration;

    let mut child = Command::new(program)
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .creation_flags(0x08000000) // CREATE_NO_WINDOW: sem console preto
        .spawn()?;

    let (tx, rx) = mpsc::channel::<ProcLine>();
    let mut handles = Vec::new();
    if let Some(out) = child.stdout.take() {
        handles.push(spawn_reader(out, ProcStream::Stdout, tx.clone()));
    }
    if let Some(err) = child.stderr.take() {
        handles.push(spawn_reader(err, ProcStream::Stderr, tx.clone()));
    }
    drop(tx);

    loop {
        // Drena o que as threads leitoras ja capturaram (ordem temporal aproximada).
        for line in rx.try_iter() {
            sink(line);
        }
        match child.try_wait()? {
            Some(status) => {
                // Processo terminou: espera as leitoras (EOF apos o filho sair)
                // e drena o restante para nao perder nenhuma linha.
                for h in handles {
                    let _ = h.join();
                }
                for line in rx.try_iter() {
                    sink(line);
                }
                return Ok(status.code().unwrap_or(-1));
            }
            None => {
                if cancel.load(Ordering::SeqCst) {
                    let _ = child.kill();
                    let status = child.wait()?;
                    for h in handles {
                        let _ = h.join();
                    }
                    for line in rx.try_iter() {
                        sink(line);
                    }
                    return Ok(status.code().unwrap_or(-1));
                }
                std::thread::sleep(Duration::from_millis(50));
            }
        }
    }
}

/// Captura a saida completa (stdout) de um comando rapido (ex.: `yt-dlp --version`).
pub fn run_capture_stdout(program: &Path, args: &[String]) -> io::Result<String> {
    use std::os::windows::process::CommandExt;
    use std::process::{Command, Stdio};

    let out = Command::new(program)
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .creation_flags(0x08000000) // CREATE_NO_WINDOW: sem console preto
        .output()?;
    Ok(String::from_utf8_lossy(&out.stdout).into_owned())
}

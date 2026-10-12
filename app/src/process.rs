//! Process spawning WITHOUT console window and output streaming/capture.

use std::io;
use std::path::Path;
use std::sync::atomic::AtomicBool;

/// Origin stream of a captured line.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProcStream {
    Stdout,
    Stderr,
}

/// A line of process output (decoded with UTF-8 lossy).
#[derive(Clone, Debug)]
pub struct ProcLine {
    pub text: String,
}

/// Reads a pipe until EOF separated by `\n` (does NOT split on `\r`, as yt-dlp
/// is executed with `--newline`) and sends each decoded line.
fn spawn_reader(
    pipe: impl std::io::Read + Send + 'static,
    _stream: ProcStream,
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
            if tx.send(ProcLine { text }).is_err() {
                break; // receiver was dropped
            }
        }
    })
}

/// Runs `program` with `args` (NO shell, NO console window) consuming stdout+stderr
/// line by line and invoking `sink` for each (approximate temporal order; equivalent
/// to `2>&1` in .bat). If `cancel` becomes true, terminates the process.
/// Returns exit code. Does not break on `\r` (use `--newline` on yt-dlp).
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
        .creation_flags(0x08000000) // CREATE_NO_WINDOW: prevents black console popup
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
        // Drain lines buffered by reader threads (approximate temporal order).
        for line in rx.try_iter() {
            sink(line);
        }
        match child.try_wait()? {
            Some(status) => {
                // Process finished: wait for reader threads (EOF after child exits)
                // and drain remainder so no output lines are lost.
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

/// Captures full stdout output of a quick command (e.g. `yt-dlp --version`).
pub fn run_capture_stdout(program: &Path, args: &[String]) -> io::Result<String> {
    use std::os::windows::process::CommandExt;
    use std::process::{Command, Stdio};

    let out = Command::new(program)
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .creation_flags(0x08000000) // CREATE_NO_WINDOW: prevents black console popup
        .output()?;
    Ok(String::from_utf8_lossy(&out.stdout).into_owned())
}

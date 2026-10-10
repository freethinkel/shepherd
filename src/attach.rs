//! `shepherd attach <pane>`: the process inside a Ghostty surface. Bridges the surface PTY to
//! `herdr terminal session control` (JSON lines: base64 ANSI frames out, input/resize in).
use base64::Engine;
use base64::engine::general_purpose::STANDARD as B64;
use serde_json::{Value, json};
use std::io::{BufRead, BufReader, Read, Write};
use std::path::Path;
use std::process::{ChildStdin, Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

type Sink = Arc<Mutex<Option<ChildStdin>>>;

pub fn command_line(exe: &Path, pane: &str) -> String {
    format!("{} attach {}", quote(&exe.to_string_lossy()), quote(pane))
}

fn quote(s: &str) -> String {
    format!("'{}'", s.replace('\'', r"'\''"))
}

pub fn run(pane: &str) -> anyhow::Result<()> {
    let herdr = std::env::var("SHEPHERD_HERDR").unwrap_or_else(|_| "herdr".into());
    let size = winsize().unwrap_or(Size { cols: 80, rows: 24, cell_w: 0, cell_h: 0 });
    let mut child = Command::new(herdr)
        .args(["terminal", "session", "control", pane])
        .args(["--cols", &size.cols.to_string(), "--rows", &size.rows.to_string()])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()?;
    let sink: Sink = Arc::new(Mutex::new(child.stdin.take()));
    let _raw = RawMode::enable();

    let input = sink.clone();
    std::thread::spawn(move || {
        let mut buf = [0u8; 4096];
        let mut stdin = std::io::stdin().lock();
        while let Ok(n @ 1..) = stdin.read(&mut buf) {
            send(&input, json!({"type": "terminal.input", "bytes": B64.encode(&buf[..n])}));
        }
        input.lock().unwrap().take(); // EOF: close herdr's stdin so it ends the session
    });

    watch_resize(sink.clone());

    let mut stdout = std::io::stdout().lock();
    for line in BufReader::new(child.stdout.take().unwrap()).lines() {
        let Ok(record) = serde_json::from_str::<Value>(&line?) else { continue };
        match record["type"].as_str() {
            Some("terminal.frame") => {
                stdout.write_all(&B64.decode(record["bytes"].as_str().unwrap_or(""))?)?;
                stdout.flush()?;
            }
            Some("terminal.closed") => break,
            _ => {}
        }
    }
    let _ = child.kill();
    let _ = child.wait();
    Ok(())
}

fn send(sink: &Sink, record: Value) {
    if let Some(stdin) = sink.lock().unwrap().as_mut() {
        let _ = writeln!(stdin, "{record}");
    }
}

static WINCH: AtomicBool = AtomicBool::new(false);

extern "C" fn on_winch(_: libc::c_int) {
    WINCH.store(true, Ordering::Relaxed);
}

// ponytail: a flag polled every 50 ms; a self-pipe if resize latency ever matters.
fn watch_resize(sink: Sink) {
    unsafe { libc::signal(libc::SIGWINCH, on_winch as extern "C" fn(libc::c_int) as libc::sighandler_t) };
    std::thread::spawn(move || loop {
        std::thread::sleep(Duration::from_millis(50));
        if WINCH.swap(false, Ordering::Relaxed) {
            if let Some(s) = winsize() {
                send(&sink, json!({"type": "terminal.resize", "cols": s.cols, "rows": s.rows,
                                   "cell_width_px": s.cell_w, "cell_height_px": s.cell_h}));
            }
        }
    });
}

struct Size {
    cols: u16,
    rows: u16,
    cell_w: u32,
    cell_h: u32,
}

fn winsize() -> Option<Size> {
    let mut ws: libc::winsize = unsafe { std::mem::zeroed() };
    if unsafe { libc::ioctl(0, libc::TIOCGWINSZ, &mut ws) } != 0 || ws.ws_col == 0 {
        return None;
    }
    Some(Size {
        cols: ws.ws_col,
        rows: ws.ws_row,
        cell_w: u32::from(ws.ws_xpixel) / u32::from(ws.ws_col),
        cell_h: u32::from(ws.ws_ypixel) / u32::from(ws.ws_row.max(1)),
    })
}

/// Raw mode on the surface PTY so every byte (ctrl keys, escape sequences) reaches herdr untouched.
struct RawMode(libc::termios);

impl RawMode {
    fn enable() -> Option<Self> {
        unsafe {
            if libc::isatty(0) == 0 {
                return None;
            }
            let mut t: libc::termios = std::mem::zeroed();
            libc::tcgetattr(0, &mut t);
            let saved = t;
            libc::cfmakeraw(&mut t);
            libc::tcsetattr(0, libc::TCSANOW, &t);
            Some(Self(saved))
        }
    }
}

impl Drop for RawMode {
    fn drop(&mut self) {
        unsafe { libc::tcsetattr(0, libc::TCSANOW, &self.0) };
    }
}

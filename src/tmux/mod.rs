//! tmux as Shepherd's backend: a session is a workspace, a window a tab and a pane a pane, and these are the sessions
//! `tmux attach` shows. Each command is one run of `tmux`. Each session has a control client ([`control`]), which streams
//! what its panes write and says when something changes; a pane's emulator reads those bytes as a terminal would.
//! tmux sizes a window, not its panes, so a pane's terminal is the size tmux gives it, however much room it has.
mod api;
mod control;
mod layout;
pub use api::Tmux;

use crate::backend::Terminal;
use anyhow::Context;
use control::{Control, Hub};
use std::path::PathBuf;
use std::process::Command;
use std::sync::Arc;

/// Bytes of one `send-keys`: a control client's line has to stay a line.
const KEYS: usize = 256;

/// `tmux`: `SHEPHERD_TMUX`, else on PATH, else where installers put it.
fn binary() -> Option<PathBuf> {
    std::env::var_os("SHEPHERD_TMUX").map(PathBuf::from).or_else(|| crate::herdr::find_program("tmux"))
}

/// Whether tmux can be offered. Not in a fixture (`SHEPHERD_MOCK`), whose window shows only what it holds.
pub fn installed() -> bool {
    crate::mock::dir().is_none() && binary().is_some()
}

/// `tmux` on the user's server, or the one `SHEPHERD_TMUX_SOCKET` names. UTF-8 always: an app opened from the Dock has
/// no locale. Not as a client of the tmux Shepherd may run in.
fn command() -> anyhow::Result<Command> {
    let mut command = crate::process::command(binary().context("tmux isn't installed")?);
    command.arg("-u").env_remove("TMUX").env_remove("TMUX_PANE");
    if let Ok(socket) = std::env::var("SHEPHERD_TMUX_SOCKET") {
        command.args(["-L", &socket]);
    }
    Ok(command)
}

/// One tmux command, or several with `;` between them.
async fn tmux(args: &[&str]) -> anyhow::Result<String> {
    let mut command = tokio::process::Command::from(command()?);
    crate::process::answer(command.args(args).output().await.context("cannot run tmux")?)
}

/// [`tmux`] from a thread of its own.
fn blocking(args: &[&str]) -> anyhow::Result<String> {
    crate::process::answer(command()?.args(args).output().context("cannot run tmux")?)
}

/// `bytes` as `send-keys` takes them from a control client, in order: raw, whatever they are.
fn keys(pane: &str, bytes: &[u8]) -> Vec<String> {
    let line = |chunk: &[u8]| format!("send-keys -t {pane} -H {}", chunk.iter().map(|b| format!("{b:02x}")).collect::<Vec<_>>().join(" "));
    bytes.chunks(KEYS).map(line).collect()
}

/// A pane's stream. Dropped, the pane detaches and the program runs on.
struct Attached {
    control: Arc<Control>,
    hub: Arc<Hub>,
    pane: String,
    route: u64,
}

impl Terminal for Attached {
    fn input(&self, bytes: &[u8]) {
        for line in keys(&self.pane, bytes) {
            self.control.send(line);
        }
    }

    /// A pane has the size tmux lays it out at: see [`crate::backend::Mux::fit`].
    fn resize(&self, _cols: u16, _rows: u16, _cell_px: (u32, u32)) {}
}

impl Drop for Attached {
    fn drop(&mut self) {
        self.hub.leave(&self.pane, self.route);
    }
}

/// What a new emulator is told before a pane's text: what its program had set, which the text doesn't say. `facts`:
/// [`FACTS`]'s line.
// ponytail: tmux has no format for bracketed paste, so a program that set it before attaching goes without until it says so again
fn modes(facts: &[&str]) -> Vec<u8> {
    let on = |at: usize| facts.get(at) == Some(&"1");
    let mut out = String::new();
    for (at, mode) in [(5, "?1"), (6, "?1000"), (7, "?1002"), (8, "?1003"), (9, "?1006"), (10, "?1005")] {
        if on(at) {
            out.push_str(&format!("\x1b[{mode}h"));
        }
    }
    if on(11) {
        out.push_str("\x1b=");
    }
    out.into_bytes()
}

/// The pane as a format: `pane_width`, `pane_height`, the cursor, the screen and the modes [`modes`] reads.
const FACTS: &str = "#{pane_width}\t#{pane_height}\t#{cursor_x}\t#{cursor_y}\t#{alternate_on}\t#{keypad_cursor_flag}\t#{mouse_standard_flag}\t#{mouse_button_flag}\t#{mouse_any_flag}\t#{mouse_sgr_flag}\t#{mouse_utf8_flag}\t#{keypad_flag}\t#{cursor_flag}";

/// `capture-pane`'s lines as bytes for an emulator: tmux ends each with a newline only.
fn lines(captured: &str) -> Vec<u8> {
    captured.trim_end_matches('\n').replace('\n', "\r\n").into_bytes()
}

/// What an emulator needs to show a pane as tmux has it now: `main` is its text, or on the alternate screen the normal
/// screen's, with `alternate` then its text.
fn restored(facts: &[&str], main: &str, alternate: Option<&str>) -> Vec<u8> {
    let mut out = lines(main);
    if let Some(screen) = alternate {
        out.extend_from_slice(b"\x1b[?1049h\x1b[H");
        out.extend(lines(screen));
    }
    out.extend(modes(facts));
    let at = |i: usize| facts.get(i).and_then(|v| v.parse::<u16>().ok()).unwrap_or(0) + 1;
    out.extend(format!("\x1b[{};{}H", at(3), at(2)).into_bytes());
    if facts.get(12) == Some(&"0") {
        out.extend_from_slice(b"\x1b[?25l");
    }
    out
}

/// Sends route `id` the pane as tmux has it now. Blocks: it runs off the reader of tmux's stream.
fn replay(hub: &Hub, pane: &str, id: u64) -> anyhow::Result<()> {
    let capture = |flags: &[&str]| blocking(&[&["capture-pane", "-p", "-e", "-J", "-t", pane], flags].concat());
    loop {
        let facts = blocking(&["display-message", "-p", "-t", pane, FACTS])?;
        let facts: Vec<&str> = facts.split('\t').collect();
        let alternate = facts.get(4) == Some(&"1");
        let main = if alternate { capture(&["-a"]) } else { capture(&["-S", "-"]) }?;
        let screen = alternate.then(|| capture(&[])).transpose()?;
        let size = |at: usize| facts.get(at).and_then(|f| f.parse().ok()).unwrap_or(0);
        if hub.replayed(pane, id, (size(0), size(1)), restored(&facts, &main, screen.as_deref())) {
            return Ok(());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keys_go_raw_and_in_short_lines() {
        assert_eq!(keys("%3", b"a\r"), ["send-keys -t %3 -H 61 0d"]);
        assert_eq!(keys("%3", "ё".as_bytes()), ["send-keys -t %3 -H d1 91"], "a key's bytes, whichever they are");
        assert_eq!(keys("%3", &[b'x'; KEYS + 1]).len(), 2);
    }

    #[test]
    fn a_pane_comes_back_as_it_was() {
        let facts = ["80", "24", "4", "1", "0", "1", "0", "0", "0", "1", "0", "1", "0", "$0"];
        let shown = String::from_utf8(restored(&facts, "one\ntwo\n", None)).unwrap();
        assert_eq!(shown, "one\r\ntwo\x1b[?1h\x1b[?1006h\x1b=\x1b[2;5H\x1b[?25l");
        let facts = ["80", "24", "0", "0", "1", "0", "0", "0", "0", "0", "0", "0", "1", "$0"];
        let shown = String::from_utf8(restored(&facts, "shell", Some("vim"))).unwrap();
        assert_eq!(shown, "shell\x1b[?1049h\x1b[Hvim\x1b[1;1H", "the normal screen stays under the alternate one");
    }
}

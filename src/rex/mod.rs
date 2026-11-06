//! Rex (Superlogical) as Shepherd's backend, beside herdr and herdr-core. Its server already keeps sessions, windows
//! and split trees, which are Shepherd's workspaces, tabs and panes, so [`api`] only translates, and these are the
//! same sessions Rex.app shows. A pane's stream starts with a snapshot of the terminal; after it come its program's
//! own bytes, which the pane's emulator reads as a terminal would.
mod api;
mod conn;
pub mod facts;
#[cfg(unix)]
mod tunnel;
mod ws;
pub use api::Rex;

use crate::backend::{SessionEvent, Terminal};
use anyhow::{Context, anyhow, bail};
use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD as B64;
use conn::Control;
use futures::channel::mpsc::{UnboundedReceiver, UnboundedSender, unbounded};
use serde_json::{Value, json};
use std::path::PathBuf;
use std::sync::mpsc;
use std::time::Duration;

const TERMINAL: &str = "com.superlogical.terminal";
const STREAM: &str = "rex.data.terminal.v1";
/// Bytes of one `write` call: a larger message than the server's limit would close the connection.
const WRITE: usize = 64 * 1024;

/// Where a Rex server answers: a socket on this machine, or a host's address on the network.
#[derive(Debug, Clone, PartialEq)]
pub enum Endpoint {
    Socket(PathBuf),
    Net { host: String, port: u16, tls: bool },
}

impl Endpoint {
    /// The server here: `REX_SERVER` as a `rex` command reads it, else where Rex.app's server listens.
    pub(super) fn local() -> anyhow::Result<Vec<Endpoint>> {
        if let Ok(server) = std::env::var("REX_SERVER") {
            return Endpoint::parse(&server);
        }
        if !cfg!(target_os = "macos") {
            bail!("Rex's server is only found on macOS: set REX_SERVER");
        }
        Ok(vec![Endpoint::Socket(crate::settings::home().join("Library/Application Support/rex/server.sock"))])
    }

    /// An address as `rex -S` takes one, as the ways to try it in turn: `unix:///path` (escaped as a URL) or a path,
    /// `http://host:port`, `https://host[:port]`, or a bare host, which is HTTPS on 443, then on 7297, then HTTP on
    /// 7297.
    pub fn parse(address: &str) -> anyhow::Result<Vec<Endpoint>> {
        let address = address.trim();
        if let Some(path) = address.strip_prefix("unix://") {
            return Ok(vec![Endpoint::Socket(percent_encoding::percent_decode_str(path).decode_utf8()?.into_owned().into())]);
        }
        if address.starts_with('/') {
            return Ok(vec![Endpoint::Socket(address.into())]);
        }
        let net = |host: &str, port, tls| Endpoint::Net { host: host.to_string(), port, tls };
        if !address.contains("://") {
            let host = address.trim_end_matches('/');
            if host.is_empty() || host.contains(['/', '@', ':']) {
                bail!("not a Rex server's address: {address}");
            }
            return Ok(vec![net(host, 443, true), net(host, 7297, true), net(host, 7297, false)]);
        }
        let uri: http::Uri = address.parse().with_context(|| format!("not a Rex server's address: {address}"))?;
        let tls = match uri.scheme_str() {
            Some("https") => true,
            Some("http") => false,
            _ => bail!("not a Rex server's address: {address}"),
        };
        let authority = uri.authority().with_context(|| format!("no host in {address}"))?;
        // ponytail: a login server's account (`https://root@host`) isn't passed on yet, so it is refused, not ignored
        if authority.as_str().contains('@') {
            bail!("choosing an account on a login server isn't supported yet: {address}");
        }
        Ok(vec![net(authority.host(), authority.port_u16().unwrap_or(if tls { 443 } else { 80 }), tls)])
    }

    pub(super) fn is_local(&self) -> bool {
        matches!(self, Endpoint::Socket(_))
    }
}

/// One of `rex hosts`: another Rex server, by its label.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Host {
    pub id: String,
    pub label: String,
    /// Its addresses, as `rex -S` takes them, tried in turn.
    pub endpoints: Vec<String>,
    #[serde(default)]
    pub icon: Option<String>,
}

impl Host {
    /// The ways to reach it, in the order its addresses are listed; an address that isn't one is left out.
    pub fn reach(&self) -> Vec<Endpoint> {
        self.endpoints.iter().filter_map(|a| Endpoint::parse(a).ok()).flatten().collect()
    }
}

/// `rex hosts remove`: the host is forgotten; its server runs on there.
pub fn remove_host(label: &str) -> anyhow::Result<()> {
    let binary = binary().context("Rex isn't installed")?;
    let out = crate::process::command(binary).args(["hosts", "remove", label]).output().context("rex hosts remove")?;
    if !out.status.success() {
        bail!("rex hosts remove: {}", String::from_utf8_lossy(&out.stderr).trim());
    }
    Ok(())
}

/// The hosts `rex hosts` lists, from the home server and this machine's.
pub fn hosts() -> anyhow::Result<Vec<Host>> {
    #[derive(serde::Deserialize)]
    struct Listed {
        hosts: Vec<Entry>,
    }
    #[derive(serde::Deserialize)]
    struct Entry {
        host: Host,
    }
    let binary = binary().context("Rex isn't installed")?;
    let out = crate::process::command(binary).args(["hosts", "list", "--json"]).output().context("rex hosts list")?;
    if !out.status.success() {
        bail!("rex hosts list: {}", String::from_utf8_lossy(&out.stderr).trim());
    }
    Ok(serde_json::from_slice::<Listed>(&out.stdout)?.hosts.into_iter().map(|e| e.host).collect())
}

/// `rex`: `SHEPHERD_REX`, else on PATH, else inside the server Rex.app keeps, which doesn't put it on PATH.
fn binary() -> Option<PathBuf> {
    let bundled = crate::settings::home().join("Library/Application Support/com.superlogical.rex/RexServer/Versions/Current/Rex Server.app/Contents/MacOS/rex");
    let found = || crate::herdr::find_program("rex").or_else(|| bundled.is_file().then_some(bundled.clone()));
    std::env::var_os("SHEPHERD_REX").map(PathBuf::from).or_else(found)
}

/// Whether Rex can be offered. Not in a fixture (`SHEPHERD_MOCK`), whose window shows only what it holds.
pub fn installed() -> bool {
    let here = || Endpoint::local().is_ok_and(|e| matches!(e.first(), Some(Endpoint::Socket(s)) if s.exists()));
    crate::mock::dir().is_none() && (here() || binary().is_some())
}

/// Starts the server, which Rex.app otherwise does, when it isn't up. Returns once it answers or it is plain it won't.
fn start_server() -> anyhow::Result<()> {
    let binary = binary().context("Rex isn't installed")?;
    let status = crate::process::command(binary).args(["server", "start"]).status().context("cannot start Rex's server")?;
    if !status.success() {
        bail!("rex server start: {status}");
    }
    let Some(Endpoint::Socket(socket)) = Endpoint::local()?.into_iter().next() else { bail!("REX_SERVER is not a local socket") };
    for _ in 0..20 {
        if socket.exists() {
            return Ok(());
        }
        std::thread::sleep(Duration::from_millis(100));
    }
    bail!("Rex's server did not start")
}

/// A pane's stream on a block, with a client of its own, so the block can be shown twice and its claim ends with the
/// pane. Dropped, the pane detaches and the program runs on.
struct Attached {
    commands: mpsc::Sender<Command>,
}

enum Command {
    Input(Vec<u8>),
    Resize(u16, u16, (u32, u32)),
}

impl Terminal for Attached {
    fn input(&self, bytes: &[u8]) {
        let _ = self.commands.send(Command::Input(bytes.to_vec()));
    }

    fn resize(&self, cols: u16, rows: u16, cell_px: (u32, u32)) {
        let _ = self.commands.send(Command::Resize(cols, rows, cell_px));
    }
}

/// The address of a block on a pane's client.
struct Block {
    control: std::sync::Arc<Control>,
    session: String,
    block: String,
}

impl Block {
    fn call(&self, method: &str, args: Value) -> anyhow::Result<Value> {
        self.control.blocking_call(&format!("{TERMINAL}.{method}"), json!({"session_id": self.session, "block_id": self.block, "args": args}))
    }

    /// The block's size and colors, from this client, which owns it.
    fn fit(&self, cols: u16, rows: u16, cell_px: (u32, u32)) -> anyhow::Result<()> {
        let mut size = json!({"columns": cols, "rows": rows});
        if cell_px.0 > 0 && cell_px.1 > 0 {
            size["cell_width_px"] = json!(cell_px.0);
            size["cell_height_px"] = json!(cell_px.1);
        }
        self.call("resize", size)?;
        if let Some((foreground, background)) = crate::backend::colors() {
            self.call("set_theme", json!({"foreground": foreground, "background": background}))?;
        }
        Ok(())
    }

    /// Types `bytes`, one call after the other: the server may run calls side by side, and keys must stay in order.
    fn write(&self, bytes: &[u8]) -> anyhow::Result<()> {
        // ponytail: the API takes text, so a byte that isn't UTF-8 (an X10 mouse report past column 95) is replaced
        let text = String::from_utf8_lossy(bytes);
        for piece in split(&text, WRITE) {
            self.call("write", json!({"data": piece}))?;
        }
        Ok(())
    }
}

/// `text` in pieces of at most `size` bytes, cut between characters.
fn split(mut text: &str, size: usize) -> Vec<&str> {
    let mut pieces = Vec::new();
    while text.len() > size {
        let cut = (1..=size).rev().find(|at| text.is_char_boundary(*at)).unwrap_or(text.len());
        let (piece, rest) = text.split_at(cut);
        pieces.push(piece);
        text = rest;
    }
    pieces.push(text);
    pieces
}

/// Attaches to pane `pane`: a snapshot of its terminal first, then what its program writes. Blocks: run it off the
/// main thread. Unless `observe`, this pane sets the block's size, as Rex.app's does: Rex.app follows it while it lasts.
/// `server` is the one the backend's own connection reached.
fn attach(server: &Endpoint, pane: &str, cols: u16, rows: u16, cell_px: (u32, u32), observe: bool) -> anyhow::Result<(Attached, UnboundedReceiver<SessionEvent>)> {
    let (session, block) = api::ids(pane)?;
    let block = Block { control: Control::open(std::slice::from_ref(server), |_| {})?, session: session.to_string(), block: block.to_string() };
    block.control.blocking_call("session.attach", json!({"session_id": session}))?;
    if !observe {
        block.control.blocking_call("session.own_block", json!({"session_id": session, "block_ids": [block.block], "mode": "take"}))?;
        block.fit(cols, rows, cell_px)?;
    }
    let ticket = block.control.blocking_call("data.connect", json!({"session_id": session, "block_id": block.block, "data_ref_id": "terminal", "subprotocol": STREAM}))?;
    let ticket = B64.decode(ticket["ticket"].as_str().context("data.connect gave no ticket")?)?;
    let (data, mut stream) = ws::connect(server, "/api/data", STREAM)?;
    data.binary(&[&[1u8][..], &ticket[..]].concat())?;
    let welcome = stream.next().map_err(|why| anyhow!("Rex refused the stream: {why}"))?;
    let nonce = nonce(&welcome).ok_or_else(|| anyhow!("Rex refused the stream: {welcome:?}"))?;

    let (tx, rx) = unbounded();
    std::thread::Builder::new().name("rex-stream".into()).spawn(move || read_stream(stream, nonce, tx))?;
    let (commands, heard) = mpsc::channel();
    std::thread::Builder::new().name("rex-input".into()).spawn(move || {
        for command in heard {
            let done = match command {
                Command::Input(bytes) => block.write(&bytes),
                Command::Resize(..) if observe => Ok(()),
                Command::Resize(cols, rows, cell_px) => block.fit(cols, rows, cell_px),
            };
            if let Err(e) = done {
                eprintln!("shepherd: rex: {e:#}");
            }
        }
        data.close();
        block.control.close();
    })?;
    Ok((Attached { commands }, rx))
}

/// The channel nonce of a data connection's WELCOME.
fn nonce(welcome: &[u8]) -> Option<u64> {
    let bytes: [u8; 32] = welcome.try_into().ok()?;
    (bytes[0] == 0x81).then(|| u64::from_le_bytes(bytes[24..].try_into().unwrap()))
}

/// Where the snapshot at the front of `head` ends, once Rex's first message shows it (`searched`: how much of `head`
/// held none). That message follows every snapshot, and its nonce is one a program can't have printed.
fn snapshot_end(head: &[u8], searched: usize, nonce: u64) -> Option<usize> {
    let mark = format!("\x1b_rex1;{nonce};");
    let from = searched.saturating_sub(mark.len());
    head[from..].windows(mark.len()).position(|w| w == mark.as_bytes()).map(|at| from + at)
}

fn read_stream(mut stream: ws::Receiver, nonce: u64, tx: UnboundedSender<SessionEvent>) {
    let mut head = Some(Vec::new());
    loop {
        let bytes = match stream.next() {
            Ok(bytes) => bytes,
            Err(why) => {
                let _ = tx.unbounded_send(SessionEvent::Ended(why));
                return;
            }
        };
        let Some(buffer) = head.as_mut() else {
            let _ = tx.unbounded_send(SessionEvent::Frame(bytes));
            continue;
        };
        let searched = buffer.len();
        buffer.extend(bytes);
        if let Some(end) = snapshot_end(buffer, searched, nonce) {
            let rest = buffer.split_off(end);
            let _ = tx.unbounded_send(SessionEvent::Snapshot(head.take().unwrap_or_default()));
            let _ = tx.unbounded_send(SessionEvent::Frame(rest));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn addresses_read_as_rex_reads_them() {
        let net = |host: &str, port, tls| Endpoint::Net { host: host.into(), port, tls };
        assert_eq!(Endpoint::parse("unix:///Users/me/Library/Application%20Support/rex/server.sock").unwrap(), [Endpoint::Socket("/Users/me/Library/Application Support/rex/server.sock".into())]);
        assert_eq!(Endpoint::parse("https://build.tail.ts.net").unwrap(), [net("build.tail.ts.net", 443, true)]);
        assert_eq!(Endpoint::parse("http://build.tail.ts.net:7297/").unwrap(), [net("build.tail.ts.net", 7297, false)]);
        assert_eq!(Endpoint::parse("build.tail.ts.net").unwrap(), [net("build.tail.ts.net", 443, true), net("build.tail.ts.net", 7297, true), net("build.tail.ts.net", 7297, false)]);
        assert!(Endpoint::parse("https://root@build.tail.ts.net").is_err(), "an account is refused, not dropped");
        assert!(Endpoint::parse("ssh://build").is_err());
    }

    #[test]
    fn a_host_is_read_from_rex_hosts() {
        let listed = r#"{"host": {"id": "host:1", "label": "studio", "endpoints": ["https://studio.ts.net", "nonsense://x"], "icon": "server", "added_from": "mac"}, "from": "home", "held": false}"#;
        let host: Host = serde_json::from_value(serde_json::from_str::<Value>(listed).unwrap()["host"].clone()).unwrap();
        assert_eq!(host.reach(), [Endpoint::Net { host: "studio.ts.net".into(), port: 443, tls: true }], "an address that isn't one is left out");
    }

    #[test]
    fn a_stream_is_a_snapshot_up_to_rexs_first_message() {
        let mark = b"\x1b_rex1;7;{\"type\":\"resize\"}";
        let stream = [&b"GHOSTSNP binary \x1b_rex1;8;\x1b_"[..], &mark[..]].concat();
        let end = stream.len() - mark.len();
        assert_eq!(snapshot_end(&stream, 0, 7), Some(end));
        assert_eq!(snapshot_end(&stream, end + 3, 7), Some(end), "the mark may start in the bytes before this frame");
        assert_eq!(snapshot_end(&stream[..end + 4], 0, 7), None, "not whole yet");
        assert_eq!(snapshot_end(&stream, 0, 9), None, "another nonce is a program's");
    }

    #[test]
    fn the_nonce_ends_the_welcome() {
        let mut welcome = [0; 32];
        welcome[0] = 0x81;
        welcome[24] = 5;
        welcome[25] = 1;
        assert_eq!(nonce(&welcome), Some(261));
        assert_eq!(nonce(&[0x82, 1]), None, "a refusal");
    }

    #[test]
    fn long_text_is_cut_between_characters() {
        assert_eq!(split("ab", 5), ["ab"]);
        assert_eq!(split("aёb", 2), ["a", "ё", "b"]);
    }
}

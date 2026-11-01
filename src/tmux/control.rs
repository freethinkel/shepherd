//! tmux's control mode: a client per session, which streams what every pane of it writes and says what changes. What
//! panes report of themselves over OSC 7501 goes by here, since tmux doesn't read it.
use crate::agents::{Report, plain_title};
use crate::backend::SessionEvent;
use futures::channel::mpsc::UnboundedSender;
use std::collections::HashMap;
use std::io::{BufRead, BufReader, Write};
use std::process::{Child, ChildStdin, ChildStdout, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, mpsc};
use tokio::sync::watch;

/// Says what a pane's title, program and folder are, at most once a second, title first.
const WATCH: &str = "refresh-client -B 'facts:%*:#{pane_title}|#{pane_current_command}|#{pane_current_path}'";
/// A report's longest wait for its end: more than that is no report.
const LONGEST_REPORT: usize = 4096;

/// A line of control mode.
#[derive(Debug, PartialEq)]
enum Note {
    Output(String, Vec<u8>),
    Layout(String),
    /// The client is on this session now.
    Session(String),
    /// A pane's facts as [`WATCH`] says them.
    Facts(String, String),
    /// A snapshot may read differently.
    Changed,
    /// The answer to a command begins or ends.
    Begin,
    End,
}

/// `%output`'s bytes: tmux writes those under a space and `\` in octal.
fn unescape(bytes: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(bytes.len());
    let mut rest = bytes;
    while let Some((&first, tail)) = rest.split_first() {
        match tail {
            [a, b, c, ..] if first == b'\\' && [a, b, c].iter().all(|d| (b'0'..=b'7').contains(d)) => {
                out.push((u32::from(a - b'0') << 6 | u32::from(b - b'0') << 3 | u32::from(c - b'0')) as u8);
                rest = &tail[3..];
            }
            _ => {
                out.push(first);
                rest = tail;
            }
        }
    }
    out
}

fn parse(line: &[u8]) -> Option<Note> {
    let (head, rest) = line.iter().position(|b| *b == b' ').map_or((line, &[][..]), |at| (&line[..at], &line[at + 1..]));
    let text = || String::from_utf8_lossy(rest).into_owned();
    match head {
        b"%output" => {
            let at = rest.iter().position(|b| *b == b' ')?;
            Some(Note::Output(String::from_utf8_lossy(&rest[..at]).into_owned(), unescape(&rest[at + 1..])))
        }
        // the layout as shown: a zoomed pane's size is the window's
        b"%layout-change" => Some(Note::Layout(text().split(' ').nth(2)?.to_string())),
        b"%session-changed" => Some(Note::Session(text().split(' ').next()?.to_string())),
        b"%subscription-changed" => {
            let text = text();
            let (head, value) = text.split_once(" : ")?;
            Some(Note::Facts(head.rsplit(' ').next()?.to_string(), value.to_string()))
        }
        b"%begin" => Some(Note::Begin),
        b"%end" | b"%error" => Some(Note::End),
        b"%window-add" | b"%window-close" | b"%unlinked-window-add" | b"%unlinked-window-close" | b"%window-renamed" | b"%window-pane-changed" | b"%session-window-changed"
        | b"%session-renamed" | b"%sessions-changed" | b"%pane-mode-changed" => Some(Note::Changed),
        _ => None,
    }
}

/// OSC 7501 in a pane's bytes, whichever frames it falls across.
#[derive(Debug, Default)]
struct Osc {
    held: Vec<u8>,
}

impl Osc {
    const MARK: &[u8] = b"\x1b]7501;";

    /// What the reports in `bytes` and the ones before say, `k=v` joined by `:`.
    fn feed(&mut self, bytes: &[u8]) -> Vec<Vec<u8>> {
        // nearly always none, in a stream that is the size of what a terminal draws
        if self.held.is_empty() && !bytes.contains(&0x1b) {
            return Vec::new();
        }
        let mut pending = std::mem::take(&mut self.held);
        pending.extend_from_slice(bytes);
        let mut found = Vec::new();
        let mut rest = &pending[..];
        loop {
            let Some(at) = rest.windows(Self::MARK.len()).position(|w| w == Self::MARK) else {
                // the mark may be cut off at the end
                let keep = (1..Self::MARK.len()).rev().find(|n| rest.ends_with(&Self::MARK[..*n])).unwrap_or(0);
                self.held = rest[rest.len() - keep..].to_vec();
                return found;
            };
            let body = &rest[at + Self::MARK.len()..];
            let bel = body.iter().position(|b| *b == 0x07);
            let st = body.windows(2).position(|w| w == b"\x1b\\");
            let end = match (bel, st) {
                (Some(b), Some(s)) if b < s => (b, 1),
                (_, Some(s)) => (s, 2),
                (Some(b), None) => (b, 1),
                (None, None) => {
                    if rest.len() - at < LONGEST_REPORT {
                        self.held = rest[at..].to_vec();
                    }
                    return found;
                }
            };
            found.push(body[..end.0].to_vec());
            rest = &body[end.0 + end.1..];
        }
    }
}

/// One report on what is known of a pane. Parts of a program's work report with an `id`, and don't speak for it.
fn reported(report: &mut Report, body: &[u8]) {
    let body = String::from_utf8_lossy(body);
    let field = |key: &str| body.split(':').find_map(|kv| kv.split_once('=').filter(|(k, _)| *k == key)).map(|(_, v)| v.to_string());
    if field("id").is_some_and(|id| !id.is_empty()) {
        return;
    }
    (report.state, report.app) = (field("state"), field("app").or(report.app.take()));
    report.turn += 1;
}

struct Route {
    id: u64,
    session: String,
    events: UnboundedSender<SessionEvent>,
    /// Bytes that came while the pane's text was read: they come after it.
    // ponytail: bytes tmux had written before it read the text show twice; a program that redraws mends it
    held: Option<Vec<u8>>,
    /// The pane changed size during that read: it is read again.
    again: bool,
    size: (u16, u16),
}

/// What the control clients share with the backend.
pub(super) struct Hub {
    routes: Mutex<HashMap<String, Vec<Route>>>,
    osc: Mutex<HashMap<String, (Osc, Report)>>,
    /// Each pane's last facts, without an agent's spinner.
    facts: Mutex<HashMap<String, String>>,
    /// Counts what changes a snapshot.
    pub(super) news: watch::Sender<u64>,
    next: AtomicU64,
}

impl Default for Hub {
    fn default() -> Self {
        Self { routes: Mutex::default(), osc: Mutex::default(), facts: Mutex::default(), news: watch::channel(0).0, next: AtomicU64::new(1) }
    }
}

impl Hub {
    pub(super) fn say(&self) {
        self.news.send_modify(|n| *n += 1);
    }

    /// `events` gets `pane`'s text and what it writes after, once [`Self::replayed`] says so; the bytes until then wait.
    pub(super) fn join(&self, pane: &str, session: &str, events: UnboundedSender<SessionEvent>) -> u64 {
        let id = self.next.fetch_add(1, Ordering::Relaxed);
        let route = Route { id, session: session.to_string(), events, held: Some(Vec::new()), again: false, size: (0, 0) };
        self.routes.lock().unwrap().entry(pane.to_string()).or_default().push(route);
        id
    }

    /// A new emulator of `size` from `text`, then what waited. `false`: the text is out of date, so read it again.
    pub(super) fn replayed(&self, pane: &str, id: u64, size: (u16, u16), text: Vec<u8>) -> bool {
        let mut routes = self.routes.lock().unwrap();
        let Some(route) = routes.get_mut(pane).and_then(|r| r.iter_mut().find(|r| r.id == id)) else { return true };
        if std::mem::take(&mut route.again) {
            return false;
        }
        route.size = size;
        let _ = route.events.unbounded_send(SessionEvent::Restart);
        let _ = route.events.unbounded_send(SessionEvent::Size(size.0, size.1));
        let _ = route.events.unbounded_send(SessionEvent::Frame(text));
        if let Some(bytes) = route.held.take().filter(|b| !b.is_empty()) {
            let _ = route.events.unbounded_send(SessionEvent::Frame(bytes));
        }
        true
    }

    pub(super) fn leave(&self, pane: &str, id: u64) {
        if let Some(routes) = self.routes.lock().unwrap().get_mut(pane) {
            routes.retain(|r| r.id != id);
        }
    }

    /// What each pane has reported.
    pub(super) fn report(&self, pane: &str) -> Report {
        self.osc.lock().unwrap().get(pane).map(|(_, report)| report.clone()).unwrap_or_default()
    }

    /// Forgets what is known of panes that are gone.
    pub(super) fn keep(&self, shown: impl Fn(&str) -> bool) {
        self.osc.lock().unwrap().retain(|pane, _| shown(pane));
        self.facts.lock().unwrap().retain(|pane, _| shown(pane));
    }

    fn output(&self, pane: String, bytes: Vec<u8>) {
        let mut osc = self.osc.lock().unwrap();
        let (scan, report) = osc.entry(pane.clone()).or_default();
        let found = scan.feed(&bytes);
        for body in &found {
            reported(report, body);
        }
        drop(osc);
        if !found.is_empty() {
            self.say();
        }
        let mut routes = self.routes.lock().unwrap();
        for route in routes.get_mut(&pane).into_iter().flatten() {
            match &mut route.held {
                Some(held) => held.extend_from_slice(&bytes),
                None => drop(route.events.unbounded_send(SessionEvent::Frame(bytes.clone()))),
            }
        }
    }

    /// The window's layout changed. A pane with a new size is read anew: tmux reflows its screen its own way, which no
    /// byte of the stream says, and an emulator resized on its own would go on from another screen.
    fn laid_out(self: &Arc<Self>, layout: &str) {
        let Some(root) = super::layout::parse(layout) else { return };
        let mut routes = self.routes.lock().unwrap();
        for (pane, rect) in root.panes() {
            for route in routes.get_mut(&pane).into_iter().flatten().filter(|r| r.size != (rect.width, rect.height)) {
                if route.held.is_some() {
                    route.again = true;
                    continue;
                }
                route.size = (rect.width, rect.height);
                route.held = Some(Vec::new());
                let (hub, pane, id) = (self.clone(), pane.clone(), route.id);
                std::thread::spawn(move || {
                    if let Err(e) = super::replay(&hub, &pane, id) {
                        eprintln!("shepherd: tmux: {e:#}"); // the pane has gone
                    }
                });
            }
        }
        drop(routes);
        self.say();
    }

    /// An agent's spinner goes on all the time, and only what its title says is news.
    fn facts(&self, pane: String, value: &str) {
        if self.facts.lock().unwrap().insert(pane, plain_title(value).to_string()).as_deref() != Some(plain_title(value)) {
            self.say();
        }
    }

    /// The session's client is gone: so are its panes' streams.
    fn lost(&self, session: &str) {
        for route in self.routes.lock().unwrap().values().flatten().filter(|r| r.session == session) {
            let _ = route.events.unbounded_send(SessionEvent::Ended("tmux closed the connection".into()));
        }
        self.say();
    }
}

/// A control client. Dropped, it detaches, and the session stays.
pub(super) struct Control {
    lines: mpsc::Sender<String>,
    child: Mutex<Child>,
}

impl Control {
    pub(super) fn open(session: &str, hub: Arc<Hub>) -> anyhow::Result<Arc<Self>> {
        let mut child = super::command()?.args(["-C", "attach", "-t", session]).stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::null()).spawn()?;
        let (input, output) = (child.stdin.take(), child.stdout.take());
        let (Some(mut input), Some(output)) = (input, output) else { anyhow::bail!("tmux gave no pipes") };
        let (lines, heard) = mpsc::channel();
        std::thread::Builder::new().name("tmux-input".into()).spawn(move || write_loop(heard, &mut input))?;
        let id = session.to_string();
        std::thread::Builder::new().name("tmux-control".into()).spawn(move || read_loop(output, &id, &hub))?;
        let control = Self { lines, child: Mutex::new(child) };
        control.send(WATCH.to_string());
        Ok(Arc::new(control))
    }

    /// A line of tmux's commands, in order; what it answers is dropped.
    pub(super) fn send(&self, line: String) {
        let _ = self.lines.send(line);
    }

    /// Whether its client still runs.
    pub(super) fn alive(&self) -> bool {
        matches!(self.child.lock().unwrap().try_wait(), Ok(None))
    }
}

impl Drop for Control {
    fn drop(&mut self) {
        let child = self.child.get_mut().unwrap();
        let _ = child.kill();
        let _ = child.wait();
    }
}

fn write_loop(lines: mpsc::Receiver<String>, input: &mut ChildStdin) {
    for line in lines {
        if writeln!(input, "{line}").is_err() {
            return;
        }
    }
}

fn read_loop(output: ChildStdout, session: &str, hub: &Arc<Hub>) {
    let mut output = BufReader::new(output);
    let (mut line, mut answering) = (Vec::new(), false);
    loop {
        line.clear();
        if !matches!(output.read_until(b'\n', &mut line), Ok(n) if n > 0) {
            break;
        }
        let text = line.strip_suffix(b"\n").unwrap_or(&line);
        let text = text.strip_suffix(b"\r").unwrap_or(text);
        match parse(text) {
            Some(Note::Begin) => answering = true,
            Some(Note::End) => answering = false,
            _ if answering => {}
            Some(Note::Output(pane, bytes)) => hub.output(pane, bytes),
            Some(Note::Layout(layout)) => hub.laid_out(&layout),
            Some(Note::Facts(pane, value)) => hub.facts(pane, &value),
            Some(Note::Changed) => hub.say(),
            // the session it was on is gone, and tmux moved it on: that one has a client of its own
            Some(Note::Session(id)) if id != session => break,
            Some(Note::Session(_)) | None => {}
        }
    }
    hub.lost(session);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn output_comes_out_as_its_bytes() {
        assert_eq!(unescape(br"hi\015\012\033[0m\134 \777x\01"), b"hi\r\n\x1b[0m\\ \xffx\\01");
        assert_eq!(parse(b"%output %3 a\\040b"), Some(Note::Output("%3".into(), b"a b".to_vec())));
        assert_eq!(parse(b"%output %3 \xd1\x91"), Some(Note::Output("%3".into(), "ё".as_bytes().to_vec())), "tmux leaves what is past ASCII as it is");
    }

    #[test]
    fn control_mode_is_read_line_by_line() {
        assert_eq!(parse(b"%begin 1793523477 284 0"), Some(Note::Begin));
        assert_eq!(parse(b"%error 1793523477 284 0"), Some(Note::End));
        assert_eq!(parse(b"%layout-change @1 a29e,90x20,0,0{1} b29e,90x20,0,0,1 *"), Some(Note::Layout("b29e,90x20,0,0,1".into())));
        assert_eq!(parse(b"%session-changed $0 probe"), Some(Note::Session("$0".into())));
        assert_eq!(parse(b"%window-pane-changed @0 %3"), Some(Note::Changed));
        assert_eq!(parse(b"%subscription-changed facts $0 @0 1 %3 : a : b|zsh|/tmp"), Some(Note::Facts("%3".into(), "a : b|zsh|/tmp".into())));
        assert_eq!(parse(b"%client-detached /dev/ttys1"), None);
    }

    #[test]
    fn a_report_is_found_across_frames() {
        let mut osc = Osc::default();
        assert!(osc.feed(b"plain text").is_empty());
        assert!(osc.feed(b"before\x1b]75").is_empty(), "cut in the mark");
        assert!(osc.feed(b"01;state=wor").is_empty(), "cut in the report");
        assert_eq!(osc.feed(b"king:app=x\x1b\\after\x1b]7501;state=done\x07"), [b"state=working:app=x".to_vec(), b"state=done".to_vec()]);
        assert!(osc.held.is_empty());
        assert!(osc.feed(b"\x1b[0m").is_empty(), "another sequence is not held on to");
        assert!(osc.held.is_empty());
    }

    #[test]
    fn a_report_that_never_ends_goes() {
        let mut osc = Osc::default();
        osc.feed(b"\x1b]7501;");
        osc.feed(&vec![b'x'; LONGEST_REPORT]);
        assert!(osc.held.is_empty());
    }

    #[test]
    fn a_report_says_how_the_program_is() {
        let mut report = Report::default();
        reported(&mut report, b"state=working:app=claude-code");
        assert_eq!((report.state.as_deref(), report.app.as_deref(), report.turn), (Some("working"), Some("claude-code"), 1));
        reported(&mut report, b"state=done");
        assert_eq!((report.state.as_deref(), report.app.as_deref(), report.turn), (Some("done"), Some("claude-code"), 2), "its name stays");
        reported(&mut report, b"id=build/test:state=working");
        assert_eq!(report.state.as_deref(), Some("done"), "a part of its work");
    }

    #[test]
    fn a_spinning_title_is_not_news() {
        let hub = Hub::default();
        let news = hub.news.subscribe();
        hub.facts("%1".into(), "⠂ fixing it|node|/repo");
        assert!(news.has_changed().unwrap());
        let mut news = hub.news.subscribe();
        news.borrow_and_update();
        hub.facts("%1".into(), "⠐ fixing it|node|/repo");
        assert!(!news.has_changed().unwrap());
        hub.facts("%1".into(), "⠐ fixing it|node|/elsewhere");
        assert!(news.has_changed().unwrap());
    }

    #[test]
    fn what_a_pane_writes_while_it_is_read_comes_after() {
        let hub = Hub::default();
        let (tx, mut rx) = futures::channel::mpsc::unbounded();
        let id = hub.join("%1", "$0", tx);
        hub.output("%1".into(), b"late".to_vec());
        assert!(rx.try_recv().is_err(), "the text goes first");
        assert!(hub.replayed("%1", id, (80, 24), b"text".to_vec()));
        assert!(matches!(rx.try_recv(), Ok(SessionEvent::Restart)));
        assert!(matches!(rx.try_recv(), Ok(SessionEvent::Size(80, 24))));
        assert!(matches!(rx.try_recv(), Ok(SessionEvent::Frame(b)) if b == b"text"));
        assert!(matches!(rx.try_recv(), Ok(SessionEvent::Frame(b)) if b == b"late"));
        hub.output("%1".into(), b"live".to_vec());
        assert!(matches!(rx.try_recv(), Ok(SessionEvent::Frame(b)) if b == b"live"));
        hub.output("%2".into(), b"another pane's".to_vec());
        assert!(rx.try_recv().is_err());
    }

    #[test]
    fn a_pane_resized_while_it_is_read_is_read_again() {
        let hub = Arc::new(Hub::default());
        let (tx, mut rx) = futures::channel::mpsc::unbounded();
        let id = hub.join("%1", "$0", tx);
        hub.laid_out("8206,80x24,0,0{40x24,0,0,0,39x24,41,0,1}");
        assert!(!hub.replayed("%1", id, (80, 24), b"the old size".to_vec()), "out of date");
        assert!(rx.try_recv().is_err());
        assert!(hub.replayed("%1", id, (39, 24), b"text".to_vec()));
        assert!(matches!(rx.try_recv(), Ok(SessionEvent::Restart)));
    }
}

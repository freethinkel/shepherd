//! [`Tmux`] as a backend. Ids are tmux's own: `$1` a session, `@2` a window, `%3` a pane.
use super::control::{Control, Hub};
use super::{Attached as Stream, layout, replay, tmux};
use crate::agents::{Report, plain_title};
use crate::backend::{Attached, Change, Kind, Made, Mode, Mux, Reply};
use crate::commands::Command;
use crate::model::{AgentStatus, Layout, Pane, Rect, Snapshot, Tab, Workspace, Worktree};
use crate::worktree::Checkouts;
use anyhow::{Context, anyhow, bail};
use futures::channel::mpsc::unbounded;
use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::sync::mpsc;

/// What an empty server has to be asked: with no session there is no client to say a new one came.
const EMPTY_POLL: Duration = Duration::from_secs(2);
/// Programs a pane sits at when nothing runs.
const SHELLS: [&str; 10] = ["sh", "bash", "zsh", "fish", "dash", "ksh", "tcsh", "csh", "nu", "login"];

const SESSIONS: &str = "s\t#{session_id}\t#{session_name}";
const WINDOWS: &str = "w\t#{session_id}\t#{window_id}\t#{window_active}\t#{window_zoomed_flag}\t#{window_width}\t#{window_height}\t#{window_layout}\t#{window_name}";
const PANES: &str = "p\t#{session_id}\t#{window_id}\t#{pane_id}\t#{pane_active}\t#{pane_tty}\t#{pane_current_command}\t#{pane_current_path}\t#{pane_title}";

/// The connection to the server, opened when first asked. Dropped, its clients detach and the sessions stay.
#[derive(Default)]
pub struct Tmux {
    hub: Arc<Hub>,
    /// Each session's control client.
    controls: Mutex<HashMap<String, Arc<Control>>>,
    /// The session of each tab and pane, as last seen.
    places: Mutex<HashMap<String, String>>,
    /// The session the window shows: tmux has one per client.
    focused: Mutex<Option<String>>,
    checkouts: Checkouts,
    /// What runs under each pane's program name, which often says little (Claude's is its version).
    programs: Mutex<HashMap<(String, String), Option<String>>>,
}

fn is_shell(program: &str) -> bool {
    SHELLS.contains(&program.trim_start_matches('-'))
}

/// No server, or one with no session left, answers so.
fn no_server(e: &anyhow::Error) -> bool {
    let message = e.to_string();
    ["no server running", "error connecting to", "server exited", "no current target"].iter().any(|word| message.contains(word))
}

/// The argv[0] of a terminal's foreground program.
async fn argv0(tty: &str) -> Option<String> {
    let tty = tty.strip_prefix("/dev/")?;
    let out = tokio::process::Command::from(crate::process::command("ps")).args(["-t", tty, "-o", "stat=,command="]).output().await.ok()?;
    let listed = String::from_utf8_lossy(&out.stdout);
    let foreground = listed.lines().filter_map(|l| l.trim().split_once(' ')).find(|(stat, _)| stat.contains('+'))?;
    foreground.1.split_whitespace().next().map(str::to_string)
}

/// A line of a listing, whose last field may hold a tab.
fn fields(line: &str) -> Vec<&str> {
    line.splitn(if line.starts_with('s') { 3 } else { 9 }, '\t').collect()
}

fn text(fields: &[&str], at: usize) -> String {
    fields.get(at).copied().unwrap_or_default().to_string()
}

fn cells(fields: &[&str], at: usize) -> u16 {
    fields.get(at).and_then(|f| f.parse().ok()).unwrap_or(0)
}

/// `listing`, [`SESSIONS`], [`WINDOWS`] and [`PANES`] a line each, as the window lays it out. `report` says what a pane
/// tells of itself. A window linked into two sessions shows in the first.
fn snapshot(listing: &str, focused: Option<&str>, report: impl Fn(&str, &str) -> Report, checkouts: &Checkouts) -> Snapshot {
    let lines: Vec<Vec<&str>> = listing.lines().map(fields).collect();
    let kind = |tag: &'static str| lines.iter().filter(move |f| f[0] == tag);
    let mut out = Snapshot::default();
    let mut seen = HashSet::new();
    for session in kind("s") {
        let id = text(session, 1);
        let mut number = 0;
        let mut active = None;
        for window in kind("w").filter(|w| text(w, 1) == id) {
            let tab = text(window, 2);
            if !seen.insert(tab.clone()) {
                continue;
            }
            number += 1;
            let panes: Vec<&Vec<&str>> = kind("p").filter(|p| text(p, 2) == tab && text(p, 1) == id).collect();
            for pane in &panes {
                let pane_id = text(pane, 3);
                let cwd = Some(text(pane, 7)).filter(|c| !c.is_empty());
                out.panes.push(Pane {
                    terminal_id: pane_id.clone(),
                    tab_id: tab.clone(),
                    workspace_id: id.clone(),
                    terminal_title_stripped: Some(plain_title(pane.get(8).copied().unwrap_or_default()).to_string()).filter(|t| !t.is_empty()),
                    cwd: cwd.clone(),
                    foreground_cwd: cwd,
                    report: Some(report(&pane_id, &text(pane, 6))),
                    pane_id,
                    ..Pane::default()
                });
            }
            let focused_pane = panes.iter().find(|p| text(p, 4) == "1").map_or_else(String::new, |p| text(p, 3));
            if text(window, 3) == "1" {
                active = Some((tab.clone(), focused_pane.clone()));
            }
            let root = layout::parse(window.get(7).copied().unwrap_or_default());
            out.layouts.push(Layout {
                tab_id: tab.clone(),
                zoomed: text(window, 4) == "1",
                area: Rect { x: 0, y: 0, width: cells(window, 5), height: cells(window, 6) },
                panes: root.as_ref().map(|r| r.laid(&focused_pane)).unwrap_or_default(),
                splits: root.as_ref().map(layout::Node::splits).unwrap_or_default(),
                focused_pane_id: focused_pane,
                minimized: Vec::new(),
            });
            out.tabs.push(Tab { tab_id: tab, workspace_id: id.clone(), label: text(window, 8), number, agent_status: AgentStatus::Unknown });
        }
        if number == 0 {
            continue; // all its windows show in another session
        }
        let name = session.get(2).copied().unwrap_or_default();
        // tmux numbers a session nobody named: its folder says more
        let first = out.panes.iter().find(|p| p.workspace_id == id).and_then(|p| p.cwd.as_deref()).and_then(|c| c.rsplit('/').find(|p| !p.is_empty()));
        let label = if name.parse::<u32>().is_ok() { first.unwrap_or(name) } else { name };
        let at = out.workspaces.len() as u32 + 1;
        if focused == Some(id.as_str()) || (focused.is_none() && out.focused_workspace_id.is_none()) {
            out.focused_workspace_id = Some(id.clone());
            out.focused_tab_id = active.as_ref().map(|(tab, _)| tab.clone());
            out.focused_pane_id = active.as_ref().map(|(_, pane)| pane.clone());
        }
        out.workspaces.push(Workspace {
            label: label.to_string(),
            number: at,
            active_tab_id: active.map(|(tab, _)| tab),
            agent_status: AgentStatus::Unknown,
            worktree: checkouts.get(&id),
            workspace_id: id,
        });
    }
    out
}

impl Tmux {
    /// The client of `session`, opened if it has none.
    fn control(&self, session: &str) -> anyhow::Result<Arc<Control>> {
        let mut controls = self.controls.lock().unwrap();
        if let Some(control) = controls.get(session).filter(|c| c.alive()) {
            return Ok(control.clone());
        }
        let control = Control::open(session, self.hub.clone())?;
        controls.insert(session.to_string(), control.clone());
        Ok(control)
    }

    /// Clients for these sessions and none for the rest.
    fn follow(&self, sessions: &[String]) {
        self.controls.lock().unwrap().retain(|session, _| sessions.contains(session));
        for session in sessions {
            if let Err(e) = self.control(session) {
                eprintln!("shepherd: tmux: {e:#}");
            }
        }
    }

    fn session_of(&self, place: &str) -> Option<String> {
        self.places.lock().unwrap().get(place).cloned()
    }

    /// What `pane` runs by its `name`: a shell runs nothing.
    async fn foreground(&self, pane: &str, tty: &str, name: &str) -> Option<String> {
        if is_shell(name) || name.is_empty() {
            return None;
        }
        let key = (pane.to_string(), name.to_string());
        if let Some(known) = self.programs.lock().unwrap().get(&key) {
            return known.clone();
        }
        let found = argv0(tty).await.or(Some(name.to_string()));
        self.programs.lock().unwrap().insert(key, found.clone());
        found
    }
}

impl Mux for Tmux {
    fn kind(&self) -> Kind {
        Kind::Tmux
    }

    fn snapshot(&self) -> Reply<'_, Snapshot> {
        Box::pin(async move {
            let listing = match tmux(&["list-sessions", "-F", SESSIONS, ";", "list-windows", "-a", "-F", WINDOWS, ";", "list-panes", "-a", "-F", PANES]).await {
                Err(e) if no_server(&e) => String::new(),
                listing => listing?,
            };
            let lines: Vec<Vec<&str>> = listing.lines().map(fields).collect();
            let sessions: Vec<String> = lines.iter().filter(|f| f[0] == "s").map(|f| text(f, 1)).collect();
            self.follow(&sessions);
            // what runs in each pane, and where: a session's repo is where its first pane is
            let mut programs = HashMap::new();
            for pane in lines.iter().filter(|f| f[0] == "p") {
                let (id, name) = (text(pane, 3), text(pane, 6));
                programs.insert(id.clone(), self.foreground(&id, &text(pane, 5), &name).await);
                self.checkouts.find(&text(pane, 1), Some(text(pane, 7)).filter(|c| !c.is_empty())).await;
            }
            self.programs.lock().unwrap().retain(|(pane, _), _| programs.contains_key(pane));
            self.hub.keep(|pane| programs.contains_key(pane));
            let focused = self.focused.lock().unwrap().clone();
            let shown = snapshot(&listing, focused.as_deref(), |pane, _| Report { program: programs.get(pane).cloned().flatten(), ..self.hub.report(pane) }, &self.checkouts);
            let mut places = self.places.lock().unwrap();
            places.clear();
            places.extend(shown.tabs.iter().map(|t| (t.tab_id.clone(), t.workspace_id.clone())));
            places.extend(shown.panes.iter().map(|p| (p.pane_id.clone(), p.workspace_id.clone())));
            drop(places);
            Ok(shown)
        })
    }

    fn changes(&self, _agents: Vec<String>) -> Reply<'_, mpsc::UnboundedReceiver<Change>> {
        Box::pin(async move {
            let (tx, rx) = mpsc::unbounded_channel();
            let mut news = self.hub.news.subscribe();
            tokio::spawn(async move {
                let mut poll = tokio::time::interval(EMPTY_POLL);
                loop {
                    tokio::select! {
                        changed = news.changed() => if changed.is_err() { break },
                        _ = poll.tick() => {}
                        _ = tx.closed() => break,
                    }
                    if tx.send(Change::Any).is_err() {
                        break;
                    }
                }
            });
            Ok(rx)
        })
    }

    fn run(&self, command: Command) -> Reply<'_, Made> {
        Box::pin(async move {
            let made = match command {
                Command::NewWorkspace { cwd } => {
                    let cwd = cwd.unwrap_or_else(|| crate::settings::home().to_string_lossy().into_owned());
                    let made = tmux(&["new-session", "-d", "-P", "-F", "#{session_id}\t#{window_id}\t#{pane_id}", "-c", &cwd]).await?;
                    let fields: Vec<&str> = made.split('\t').collect();
                    let [workspace, tab, pane] = fields[..] else { bail!("tmux made {made}") };
                    Made { workspace_id: Some(workspace.into()), tab_id: Some(tab.into()), pane_id: Some(pane.into()) }
                }
                Command::NewTab { workspace_id } => {
                    let target = format!("{workspace_id}:");
                    let made = tmux(&["new-window", "-d", "-t", &target, "-P", "-F", "#{window_id}\t#{pane_id}", "-c", "#{pane_current_path}"]).await?;
                    let (tab, pane) = made.split_once('\t').context("new-window")?;
                    Made { tab_id: Some(tab.into()), pane_id: Some(pane.into()), ..Made::default() }
                }
                Command::Split { pane_id, down, cwd } => {
                    let cwd = cwd.unwrap_or_else(|| "#{pane_current_path}".into());
                    let pane = tmux(&["split-window", if down { "-v" } else { "-h" }, "-t", &pane_id, "-P", "-F", "#{pane_id}", "-c", &cwd]).await?;
                    Made { pane_id: Some(pane), ..Made::default() }
                }
                Command::CloseWorkspace { workspace_id } => {
                    tmux(&["kill-session", "-t", &workspace_id]).await?;
                    Made::default()
                }
                Command::CloseTab { tab_id } => {
                    tmux(&["kill-window", "-t", &tab_id]).await?;
                    Made::default()
                }
                Command::ClosePane { pane_id } => {
                    tmux(&["kill-pane", "-t", &pane_id]).await?;
                    Made::default()
                }
                Command::RenameWorkspace { workspace_id, label } => {
                    tmux(&["rename-session", "-t", &workspace_id, &label]).await?;
                    Made::default()
                }
                Command::RenameTab { tab_id, label } => {
                    tmux(&["rename-window", "-t", &tab_id, &label]).await?;
                    Made::default()
                }
                Command::MoveTab { tab_id, insert_index } => {
                    let session = self.session_of(&tab_id).ok_or_else(|| anyhow!("no tab {tab_id}"))?;
                    let target = format!("{session}:");
                    let windows = tmux(&["list-windows", "-t", &target, "-F", "#{window_id}"]).await?;
                    let windows: Vec<&str> = windows.lines().collect();
                    match windows.get(insert_index) {
                        Some(before) if *before == tab_id => {}
                        Some(before) => drop(tmux(&["move-window", "-d", "-b", "-s", &tab_id, "-t", before]).await?),
                        None => drop(tmux(&["move-window", "-d", "-a", "-s", &tab_id, "-t", windows.last().context("no windows")?]).await?),
                    }
                    Made::default()
                }
                Command::MoveWorkspaces { .. } => bail!("tmux keeps its sessions in the order of their names"),
                Command::FocusTab { tab_id } => {
                    *self.focused.lock().unwrap() = self.session_of(&tab_id);
                    tmux(&["select-window", "-t", &tab_id]).await?;
                    Made::default()
                }
                Command::Focus { pane_id } => {
                    *self.focused.lock().unwrap() = self.session_of(&pane_id);
                    tmux(&["select-pane", "-t", &pane_id]).await?;
                    Made::default()
                }
                Command::FocusPane { pane_id, direction } => {
                    let way = match direction {
                        crate::commands::Direction::Left => "-L",
                        crate::commands::Direction::Right => "-R",
                        crate::commands::Direction::Up => "-U",
                        crate::commands::Direction::Down => "-D",
                    };
                    tmux(&["select-pane", "-t", &pane_id, way]).await?;
                    Made::default()
                }
                Command::Zoom { pane_id } => {
                    tmux(&["resize-pane", "-Z", "-t", &pane_id]).await?;
                    Made::default()
                }
                Command::SwapPanes { source, target } => {
                    tmux(&["swap-pane", "-d", "-s", &source, "-t", &target]).await?;
                    Made::default()
                }
                Command::MovePane { pane_id, tab_id, down, target } => {
                    tmux(&["join-pane", "-d", if down { "-v" } else { "-h" }, "-s", &pane_id, "-t", target.as_deref().unwrap_or(&tab_id)]).await?;
                    Made { tab_id: Some(tab_id), ..Made::default() }
                }
                Command::MovePaneToNewTab { pane_id, workspace_id } => {
                    // a client of nothing has no session of its own
                    let session = workspace_id.or_else(|| self.session_of(&pane_id)).ok_or_else(|| anyhow!("no pane {pane_id}"))?;
                    let target = format!("{session}:");
                    Made { tab_id: Some(tmux(&["break-pane", "-d", "-s", &pane_id, "-t", &target, "-P", "-F", "#{window_id}"]).await?), ..Made::default() }
                }
                Command::SetSplitRatio { tab_id, path, ratio } => {
                    let layout = tmux(&["display-message", "-p", "-t", &tab_id, "#{window_layout}"]).await?;
                    let border = layout::parse(&layout).and_then(|root| root.border(&path)).ok_or_else(|| anyhow!("no split there"))?;
                    let moved = (ratio * f32::from(border.whole)).round() as i32 - i32::from(border.first);
                    let way = match (border.across, moved < 0) {
                        (true, false) => "-R",
                        (true, true) => "-L",
                        (false, false) => "-D",
                        (false, true) => "-U",
                    };
                    tmux(&["resize-pane", way, "-t", &border.pane, &moved.unsigned_abs().to_string()]).await?;
                    Made::default()
                }
            };
            self.hub.say(); // tmux says little of what a client does
            Ok(made)
        })
    }

    fn cwd(&self, pane: String) -> Reply<'_, Option<String>> {
        Box::pin(async move {
            let cwd = tmux(&["display-message", "-p", "-t", &pane, "#{pane_current_path}"]).await?;
            Ok(Some(cwd).filter(|c| !c.is_empty()))
        })
    }

    fn read(&self, pane: String, lines: usize, ansi: bool) -> Reply<'_, String> {
        Box::pin(async move {
            let start = format!("-{lines}");
            let mut args = vec!["capture-pane", "-p", "-J", "-t", &pane, "-S", &start];
            if ansi {
                args.push("-e");
            }
            tmux(&args).await
        })
    }

    fn program(&self, pane: String) -> Reply<'_, Option<String>> {
        Box::pin(async move {
            let name = tmux(&["display-message", "-p", "-t", &pane, "#{pane_current_command}"]).await?;
            Ok(Some(name).filter(|n| !n.is_empty() && !is_shell(n)))
        })
    }

    fn send(&self, pane: String, text: Option<String>) -> Reply<'_, ()> {
        Box::pin(async move {
            if let Some(text) = text {
                tmux(&["send-keys", "-l", "-t", &pane, "--", &crate::backend::typed(text)]).await?;
            }
            tmux(&["send-keys", "-t", &pane, "Enter"]).await.map(drop)
        })
    }

    fn attach(&self, pane: &str, _cols: u16, _rows: u16, _cell_px: (u32, u32), _mode: Mode) -> anyhow::Result<Attached> {
        let session = super::blocking(&["display-message", "-p", "-t", pane, "#{session_id}"])?;
        let control = self.control(&session)?;
        let (events, rx) = unbounded();
        // read after the stream starts and sent before it: nothing falls between the two
        let route = self.hub.join(pane, &session, events);
        replay(&self.hub, pane, route)?;
        Ok((Box::new(Stream { control, hub: self.hub.clone(), pane: pane.to_string(), route }), rx))
    }

    fn remember(&self, workspace: String, tree: Worktree) -> Reply<'_, ()> {
        self.checkouts.set(workspace, tree);
        Box::pin(async { Ok(()) })
    }

    /// All of a session's windows take the client's size: a window sized on its own would leave the others at the
    /// 80×24 of a client that has none.
    fn fit(&self, tab: &str, cols: u16, rows: u16) {
        let control = self.session_of(tab).and_then(|session| self.controls.lock().unwrap().get(&session).cloned());
        if let Some(control) = control {
            control.send(format!("refresh-client -C {cols}x{rows}"));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::backend::{Backend, SessionEvent};
    use crate::model::AgentStatus as Status;

    const LISTING: &str = "s\t$0\tshepherd\ns\t$1\t3\n\
        w\t$0\t@0\t0\t0\t120\t40\tc,120x40,0,0,0\tzsh\n\
        w\t$0\t@1\t1\t1\t120\t40\tc,120x40,0,0{60x40,0,0,1,59x40,61,0,2}\tnvim\n\
        w\t$1\t@2\t1\t0\t80\t24\tc,80x24,0,0,3\tzsh\n\
        p\t$0\t@0\t%0\t1\t/dev/ttys1\tzsh\t/repo\tmacbook\n\
        p\t$0\t@1\t%1\t0\t/dev/ttys2\t2.1.0\t/repo\t\u{2802} fixing it\n\
        p\t$0\t@1\t%2\t1\t/dev/ttys3\tzsh\t/repo/src\ttwo\twords\n\
        p\t$1\t@2\t%3\t1\t/dev/ttys4\tzsh\t/work/api\tthere";

    fn read(focused: Option<&str>) -> Snapshot {
        snapshot(LISTING, focused, |_, name| Report { program: Some(name.into()), ..Report::default() }, &Checkouts::default())
    }

    #[test]
    fn the_window_reads_tmuxs_sessions() {
        let s = read(None);
        assert_eq!(s.workspaces.iter().map(|w| w.label.as_str()).collect::<Vec<_>>(), ["shepherd", "api"], "a numbered session goes by its folder");
        assert_eq!(s.tabs.iter().map(|t| (t.label.as_str(), t.number)).collect::<Vec<_>>(), [("zsh", 1), ("nvim", 2), ("zsh", 1)]);
        assert_eq!(s.workspaces[0].active_tab_id.as_deref(), Some("@1"));
        assert_eq!((s.focused_workspace_id.as_deref(), s.focused_tab_id.as_deref(), s.focused_pane_id.as_deref()), (Some("$0"), Some("@1"), Some("%2")));
        let pane = |id: &str| s.panes.iter().find(|p| p.pane_id == id).unwrap();
        assert_eq!(pane("%1").terminal_title_stripped.as_deref(), Some("fixing it"));
        assert_eq!(pane("%2").terminal_title_stripped.as_deref(), Some("two\twords"), "a title with a tab in it");
        assert_eq!(pane("%1").report.as_ref().and_then(|r| r.program.as_deref()), Some("2.1.0"));
        assert_eq!(pane("%1").agent_status, Status::Unknown, "left to the agents' wrapper");
        let layout = s.layouts.iter().find(|l| l.tab_id == "@1").unwrap();
        assert!(layout.zoomed);
        assert_eq!(layout.focused_pane_id, "%2");
        assert!(layout.panes[1].focused && !layout.panes[0].focused);
        assert_eq!(s.pane_frames("@1").len(), 1, "the zoomed one fills it");
        assert_eq!(s.pane_frames("@0")[0].frame.w, 1.);
    }

    #[test]
    fn focusing_a_session_shows_it() {
        let s = read(Some("$1"));
        assert_eq!((s.focused_tab_id.as_deref(), s.focused_pane_id.as_deref()), (Some("@2"), Some("%3")));
    }

    #[test]
    fn a_window_linked_twice_shows_once() {
        let listing = "s\t$0\ta\ns\t$1\tb\nw\t$0\t@0\t1\t0\t80\t24\tc,80x24,0,0,0\tzsh\nw\t$1\t@0\t1\t0\t80\t24\tc,80x24,0,0,0\tzsh\n\
            p\t$0\t@0\t%0\t1\t/dev/ttys1\tzsh\t/a\tt\np\t$1\t@0\t%0\t1\t/dev/ttys1\tzsh\t/a\tt";
        let s = snapshot(listing, None, |_, _| Report::default(), &Checkouts::default());
        assert_eq!((s.workspaces.len(), s.tabs.len(), s.panes.len()), (1, 1, 1));
    }

    #[test]
    fn an_empty_server_has_nothing_to_show() {
        assert!(snapshot("", None, |_, _| Report::default(), &Checkouts::default()).workspaces.is_empty());
        assert!(no_server(&anyhow!("no server running on /tmp/tmux-501/default")));
        assert!(no_server(&anyhow!("error connecting to /tmp/tmux-501/default (No such file or directory)")));
        assert!(!no_server(&anyhow!("can't find pane: %9")));
    }

    #[test]
    fn a_shell_runs_nothing() {
        assert!(is_shell("zsh") && is_shell("-bash"));
        assert!(!is_shell("nvim"));
    }

    /// Waits for what `done` wants of the snapshots.
    async fn until(backend: &Backend, what: &str, mut done: impl FnMut(&Snapshot) -> bool) -> Snapshot {
        for _ in 0..100 {
            let shot = backend.snapshot().await.unwrap();
            if done(&shot) {
                return shot;
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
        panic!("no {what}");
    }

    /// Reads a pane's stream into an emulator until `done`.
    async fn watch(emulator: &mut crate::terminal::emulator::Emulator, stream: &mut futures::channel::mpsc::UnboundedReceiver<SessionEvent>, done: impl Fn(&crate::terminal::emulator::Emulator) -> bool) -> bool {
        use futures::StreamExt;
        let reading = async {
            while let Some(event) = stream.next().await {
                match event {
                    SessionEvent::Restart => *emulator = crate::terminal::emulator::Emulator::raw(emulator.cols() as u16, emulator.rows() as u16),
                    SessionEvent::Size(cols, rows) => emulator.resize(cols, rows),
                    SessionEvent::Frame(bytes) => {
                        emulator.feed(&bytes);
                        if done(emulator) {
                            return true;
                        }
                    }
                    _ => {}
                }
            }
            false
        };
        tokio::time::timeout(Duration::from_secs(10), reading).await.unwrap_or(false)
    }

    /// Live, on a server of its own: `SHEPHERD_TMUX_SOCKET=shepherd-live cargo test tmux::api::tests::live -- --ignored --nocapture`.
    #[tokio::test(flavor = "multi_thread")]
    #[ignore]
    async fn live() {
        assert!(std::env::var("SHEPHERD_TMUX_SOCKET").is_ok(), "run it on a server of its own: it closes what it opens");
        let backend = Backend::of(Kind::Tmux, None);
        let (b, mux) = (&backend, &backend.mux);
        assert!(b.snapshot().await.unwrap().workspaces.is_empty(), "no server is an empty one");
        let mut events = mux.changes(Vec::new()).await.unwrap();
        let repo = tempfile::tempdir().unwrap();
        let repo = std::fs::canonicalize(repo.path()).unwrap().to_string_lossy().into_owned();
        let made = mux.run(Command::NewWorkspace { cwd: Some(repo.clone()) }).await.unwrap();
        let (workspace, tab, first) = (made.workspace_id.unwrap(), made.tab_id.unwrap(), made.pane_id.unwrap());
        assert!(tokio::time::timeout(Duration::from_secs(3), events.recv()).await.unwrap().is_some(), "it said so");
        let shot = until(b, "a workspace", |s| s.workspaces.len() == 1).await;
        assert_eq!(shot.panes[0].cwd.as_deref(), Some(repo.as_str()));

        let second = mux.run(Command::Split { pane_id: first.clone(), down: false, cwd: None }).await.unwrap().pane_id.unwrap();
        let shot = until(b, "two panes", |s| s.panes.len() == 2).await;
        assert_eq!(shot.focused_pane_id.as_deref(), Some(second.as_str()), "a split takes the focus");
        assert_eq!((shot.pane_frames(&tab).len(), shot.split_frames(&tab).len()), (2, 1));

        // tmux sizes the tab, and the pane's terminal takes what it gets
        mux.fit(&tab, 121, 41);
        let shot = until(b, "the tab's size", |s| s.layouts[0].area.width == 121).await;
        assert_eq!(shot.layouts[0].area.height, 41);

        let (tmux_mux, pane) = (mux.clone(), first.clone());
        let (attached, mut stream) = tokio::task::spawn_blocking(move || tmux_mux.attach(&pane, 0, 0, (0, 0), Mode::Control)).await.unwrap().unwrap();
        attached.input(b"clear; seq 1 200; echo shepherd-$((40+2))\r");
        let mut emulator = crate::terminal::emulator::Emulator::raw(10, 10);
        let shown = |e: &crate::terminal::emulator::Emulator| (0..e.rows()).any(|row| e.row_text(row).trim() == "shepherd-42");
        assert!(watch(&mut emulator, &mut stream, shown).await, "the shell ran it and the emulator shows its output");
        let laid = shot.layouts[0].panes.iter().find(|p| p.pane_id == first).unwrap().rect;
        assert_eq!((emulator.cols(), emulator.rows()), (laid.width as usize, 41), "the size tmux gave it");
        assert!(mux.read(first.clone(), 5, false).await.unwrap().contains("shepherd-42"));

        // a new size: tmux reflows its screen, and the emulator shows that one, not one of its own
        mux.fit(&tab, 100, 30);
        until(b, "a new size", |s| s.layouts[0].area.width == 100).await;
        tokio::time::sleep(Duration::from_millis(200)).await;
        let screen = tmux(&["capture-pane", "-p", "-t", &first]).await.unwrap();
        let screen: Vec<&str> = screen.lines().map(str::trim_end).collect();
        let same = |e: &crate::terminal::emulator::Emulator| e.rows() == 30 && (0..screen.len()).all(|row| e.row_text(row).trim_end() == screen[row]);
        assert!(watch(&mut emulator, &mut stream, same).await, "tmux shows {screen:?}");

        attached.input("printf 'ё%.0s' 1 2 3; echo\r".as_bytes());
        assert!(watch(&mut emulator, &mut stream, |e| (0..e.rows()).any(|row| e.row_text(row).trim() == "ёёё")).await, "UTF-8 goes through send-keys -H");

        // a second look replays what is there, history included
        let (tmux_mux, pane) = (mux.clone(), first.clone());
        let (second_look, mut replay) = tokio::task::spawn_blocking(move || tmux_mux.attach(&pane, 0, 0, (0, 0), Mode::Observe)).await.unwrap().unwrap();
        let mut replayed = crate::terminal::emulator::Emulator::raw(10, 10);
        assert!(watch(&mut replayed, &mut replay, shown).await, "the text is there before anything new");
        assert!(replayed.history_lines() > 100, "{}", replayed.history_lines());
        drop(second_look);

        // the alternate screen comes back on it, over the normal one
        attached.input(b"printf '\\033[?1049h\\033[2J\\033[3;5Hfullscreen'; sleep 30\r");
        assert!(watch(&mut emulator, &mut stream, |e| (0..e.rows()).any(|row| e.row_text(row).contains("fullscreen"))).await);
        let (tmux_mux, pane) = (mux.clone(), first.clone());
        let (third_look, mut replay) = tokio::task::spawn_blocking(move || tmux_mux.attach(&pane, 0, 0, (0, 0), Mode::Observe)).await.unwrap().unwrap();
        let mut replayed = crate::terminal::emulator::Emulator::raw(10, 10);
        assert!(watch(&mut replayed, &mut replay, |e| e.alt_screen() && (0..e.rows()).any(|row| e.row_text(row).contains("fullscreen"))).await);
        drop(third_look);
        attached.input(b"\x03");
        attached.input(b"printf '\\033[?1049l'\r");

        // an agent's status, over OSC 7501, which tmux doesn't read
        attached.input(b"printf '\\033]7501;state=working:app=claude-code\\007'\r");
        let agent = until(b, "a reported status", |s| s.panes.iter().any(|p| p.pane_id == first && p.agent.as_deref() == Some("claude"))).await;
        assert_eq!(agent.panes.iter().find(|p| p.pane_id == first).unwrap().agent_status, Status::Working);
        assert_eq!(agent.tabs[0].agent_status, Status::Working);
        attached.input(b"printf '\\033]7501;state=done\\033\\\\'\r");
        until(b, "a finished agent", |s| s.panes.iter().any(|p| p.pane_id == first && p.agent_status == Status::Done)).await;
        backend.agents.seen(first.clone()).await.unwrap();
        until(b, "a seen agent", |s| s.panes.iter().any(|p| p.pane_id == first && p.agent_status == Status::Idle)).await;
        drop(attached);

        // a program shows, a shell doesn't
        mux.send(second.clone(), Some("sleep 30".into())).await.unwrap();
        for _ in 0..50 {
            if mux.program(second.clone()).await.unwrap().is_some() {
                break;
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
        assert_eq!(mux.program(second.clone()).await.unwrap().as_deref(), Some("sleep"));
        assert_eq!(mux.program(first.clone()).await.unwrap(), None);
        tokio::process::Command::from(super::super::command().unwrap()).args(["send-keys", "-t", &second, "C-c"]).status().await.unwrap();

        let tab2 = mux.run(Command::NewTab { workspace_id: workspace.clone() }).await.unwrap().tab_id.unwrap();
        mux.run(Command::RenameTab { tab_id: tab2.clone(), label: "logs".into() }).await.unwrap();
        until(b, "a renamed tab", |s| s.tabs.iter().any(|t| t.label == "logs")).await;
        mux.run(Command::MoveTab { tab_id: tab2.clone(), insert_index: 0 }).await.unwrap();
        let shot = until(b, "a moved tab", |s| s.tabs[0].tab_id == tab2).await;
        assert_eq!(shot.tabs_of(&workspace)[0].label, "logs");
        mux.run(Command::MoveTab { tab_id: tab2.clone(), insert_index: 2 }).await.unwrap();
        until(b, "a tab moved to the end", |s| s.tabs[1].tab_id == tab2).await;
        mux.run(Command::RenameWorkspace { workspace_id: workspace.clone(), label: "renamed".into() }).await.unwrap();
        until(b, "a renamed workspace", |s| s.workspaces[0].label == "renamed").await;
        mux.run(Command::SwapPanes { source: first.clone(), target: second.clone() }).await.unwrap();
        let shot = until(b, "swapped panes", |s| s.layouts.iter().find(|l| l.tab_id == tab).is_some_and(|l| l.panes[0].pane_id == second)).await;
        let before = shot.split_frames(&tab)[0].ratio;
        mux.run(Command::SetSplitRatio { tab_id: tab.clone(), path: Vec::new(), ratio: 0.25 }).await.unwrap();
        let shot = until(b, "a moved border", |s| s.split_frames(&tab).first().is_some_and(|f| f.ratio != before)).await;
        assert!((shot.split_frames(&tab)[0].ratio - 0.25).abs() < 0.02, "{}", shot.split_frames(&tab)[0].ratio);
        mux.run(Command::Zoom { pane_id: second.clone() }).await.unwrap();
        until(b, "a zoom", |s| s.layouts.iter().any(|l| l.tab_id == tab && l.zoomed)).await;
        mux.run(Command::Zoom { pane_id: second.clone() }).await.unwrap();
        until(b, "no zoom", |s| s.layouts.iter().any(|l| l.tab_id == tab && !l.zoomed)).await;
        mux.run(Command::FocusPane { pane_id: second.clone(), direction: crate::commands::Direction::Right }).await.unwrap();
        until(b, "the focus to the right", |s| s.layouts.iter().any(|l| l.tab_id == tab && l.focused_pane_id == first)).await;

        let moved = mux.run(Command::MovePaneToNewTab { pane_id: second.clone(), workspace_id: None }).await.unwrap().tab_id.unwrap();
        let shot = until(b, "a pane in its own tab", |s| s.panes.iter().any(|p| p.pane_id == second && p.tab_id == moved)).await;
        assert_eq!(shot.pane_frames(&moved).len(), 1);
        mux.run(Command::MovePane { pane_id: second.clone(), tab_id: tab.clone(), down: true, target: Some(first.clone()) }).await.unwrap();
        let shot = until(b, "a pane back in its tab", |s| s.panes.iter().any(|p| p.pane_id == second && p.tab_id == tab)).await;
        assert!(!shot.split_frames(&tab)[0].vertical, "stacked");
        assert!(mux.run(Command::MoveWorkspaces { workspace_ids: vec![workspace.clone()], before: None }).await.is_err(), "tmux has no such order");

        mux.run(Command::ClosePane { pane_id: second.clone() }).await.unwrap();
        until(b, "a closed pane", |s| !s.panes.iter().any(|p| p.pane_id == second)).await;
        mux.run(Command::CloseTab { tab_id: tab2.clone() }).await.unwrap();
        until(b, "a closed tab", |s| !s.tabs.iter().any(|t| t.tab_id == tab2)).await;

        // a worktree, in the real `~/.herdr/worktrees/<a temporary repo>`, which goes at the end
        for args in [&["init", "-q"][..], &["-c", "user.name=t", "-c", "user.email=t@t", "commit", "-q", "--allow-empty", "-m", "first"]] {
            assert!(std::process::Command::new("git").arg("-C").arg(&repo).args(args).status().unwrap().success());
        }
        let trees = &backend.worktrees;
        let workspace_of = |s: &Snapshot, id: &str| s.workspaces.iter().find(|w| w.workspace_id == id).cloned();
        let main = mux.run(Command::NewWorkspace { cwd: Some(repo.clone()) }).await.unwrap().workspace_id.unwrap();
        until(b, "a repo", |s| workspace_of(s, &main).is_some_and(|w| w.worktree.is_some())).await;
        let made = trees.create(main.clone(), "fix/tmux".into()).await.unwrap();
        assert!(made.tab_id.is_some(), "the window selects its tab");
        let linked = made.workspace_id.unwrap();
        let shot = until(b, "a nested worktree", |s| s.workspace_tree().iter().any(|(w, child)| w.workspace_id == linked && child.is_some())).await;
        let tree = workspace_of(&shot, &linked).unwrap().worktree.unwrap();
        assert!(tree.is_linked_worktree);
        assert_eq!(workspace_of(&shot, &linked).unwrap().label, "fix-tmux");
        let open: Vec<_> = trees.list(main.clone()).await.unwrap().into_iter().map(|w| w.open_workspace_id.unwrap_or_default()).collect();
        assert_eq!(open, [main.clone(), linked.clone()]);
        std::fs::write(std::path::Path::new(&tree.checkout_path).join("work"), "unsaved").unwrap();
        let refused = trees.remove(linked.clone(), false).await.unwrap_err().to_string();
        assert!(refused.starts_with("dirty_worktree_requires_force"), "{refused}");
        trees.remove(linked.clone(), true).await.unwrap();
        until(b, "a removed worktree", |s| workspace_of(s, &linked).is_none()).await;
        assert!(!std::path::Path::new(&tree.checkout_path).exists());
        std::fs::remove_dir(std::path::Path::new(&tree.checkout_path).parent().unwrap()).unwrap();

        mux.run(Command::CloseWorkspace { workspace_id: main }).await.unwrap();
        mux.run(Command::CloseWorkspace { workspace_id: workspace }).await.unwrap();
        until(b, "an empty server", |s| s.workspaces.is_empty()).await;
    }
}

//! What Shepherd asks of whatever runs its terminals. A [`Mux`] keeps the terminals and their layout, and is the one
//! thing a new backend has to write. Worktrees and agents go through wrappers of their own: herdr has both, so
//! [`Worktrees`] and [`Agents`] are its API there, and the others share [`crate::worktree::Git`] and
//! [`crate::agents::Own`] on top of their [`Mux`]. What only herdr has (plugins, popups, machines) asks [`Mux::herdr`].
use crate::agents::Own;
use crate::commands::Command;
use crate::herdr::Client;
use crate::model::{Pane, Snapshot, Worktree, WorktreeEntry};
use crate::worktree::Git;
use futures::channel::mpsc::UnboundedReceiver;
use futures::future::BoxFuture;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::sync::mpsc;

pub type Reply<'a, T> = BoxFuture<'a, anyhow::Result<T>>;

/// What runs the terminals. An unknown (future) value reads as herdr.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Kind {
    Hc,
    Rex,
    Tmux,
    #[default]
    #[serde(other)]
    Herdr,
}

impl Kind {
    pub const ALL: [Kind; 4] = [Kind::Herdr, Kind::Hc, Kind::Rex, Kind::Tmux];

    pub fn name(self) -> &'static str {
        match self {
            Kind::Herdr => "herdr",
            Kind::Hc => "herdr-core",
            Kind::Rex => "Rex",
            Kind::Tmux => "tmux",
        }
    }

    /// As the settings say it, which also keys what is kept for it while another is shown.
    pub fn key(self) -> &'static str {
        match self {
            Kind::Herdr => "herdr",
            Kind::Hc => "hc",
            Kind::Rex => "rex",
            Kind::Tmux => "tmux",
        }
    }

    /// What Settings → Backend says of it. Blocks: its command is asked.
    pub fn facts(self) -> Vec<Fact> {
        match self {
            Kind::Hc => crate::hc::facts::facts(),
            Kind::Rex => crate::rex::facts::facts(),
            Kind::Herdr | Kind::Tmux => Vec::new(),
        }
    }

    /// Runs a [`Job`]'s `args`: what its command said. Blocks.
    pub fn run(self, args: &[&str]) -> anyhow::Result<String> {
        match self {
            Kind::Hc => crate::hc::facts::run(args),
            Kind::Rex => crate::rex::facts::run(args),
            Kind::Herdr | Kind::Tmux => anyhow::bail!("{} has no jobs", self.name()),
        }
    }

    /// Whether it can be offered.
    pub fn installed(self) -> bool {
        match self {
            Kind::Herdr => true,
            Kind::Hc => crate::hc::installed(),
            Kind::Rex => crate::rex::installed(),
            Kind::Tmux => crate::tmux::installed(),
        }
    }
}

/// A row of Settings → Backend about a backend.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Fact {
    pub title: String,
    pub detail: String,
    pub actions: Vec<Job>,
    /// Its file, which the row can show.
    pub file: Option<std::path::PathBuf>,
}

/// A button on a [`Fact`]: its label, and what it runs of the backend's own command.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Job {
    pub label: &'static str,
    pub args: &'static [&'static str],
}

/// What a command made, which the window goes to.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Made {
    pub workspace_id: Option<String>,
    pub tab_id: Option<String>,
    pub pane_id: Option<String>,
}

impl Made {
    /// From herdr's answer: `tab.create`, `workspace.create`, `pane.split` and `pane.move` each say it their way.
    pub fn of(result: &Value) -> Self {
        let text = |value: &Value| value.as_str().map(str::to_string);
        Self {
            workspace_id: text(&result["workspace"]["workspace_id"]),
            tab_id: text(&result["tab"]["tab_id"]).or_else(|| text(&result["move_result"]["pane"]["tab_id"])),
            pane_id: text(&result["root_pane"]["pane_id"]).or_else(|| text(&result["pane"]["pane_id"])),
        }
    }
}

/// What a backend says when a snapshot may be out of date. It never says what changed but for the pane: a snapshot is
/// the one truth, taken anew.
#[derive(Debug, Clone, PartialEq)]
pub enum Change {
    Any,
    /// Another client took the focus (herdr's TUI, over ssh), and the client used last sizes the panes.
    Focused,
    /// This pane reads so now: a snapshot that has it the same stays as it is.
    Pane(Box<Pane>),
}

/// How a session holds its pane. herdr allows one controller, which types into the pane and sets its size;
/// observers only watch, so its other clients size the pane.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Mode {
    Control,
    /// Control, replacing another controller.
    Takeover,
    Observe,
}

pub enum SessionEvent {
    /// The stream starts over: a new emulator, before the frames that come next.
    Restart,
    /// Terminal bytes to feed the emulator.
    Frame(Vec<u8>),
    /// The terminal as it is, in Ghostty's snapshot format: a new emulator, before any frame (Rex).
    Snapshot(Vec<u8>),
    /// Whether the pane's app asks for mouse reporting (herdr's socket sessions only).
    MouseCapture(bool),
    /// The kitty keyboard flags the pane's app asked for (herdr's socket sessions only).
    KeyboardFlags(u16),
    /// The backend decides the terminal's size, whatever room the pane has (tmux): the frames that follow are this size.
    Size(u16, u16),
    /// The stream is over: the backend's reason, its stderr, or why it could not run.
    Ended(String),
}

/// A pane's hold on its terminal. Dropped, the pane detaches and the program runs on.
pub trait Terminal: Send {
    fn input(&self, bytes: &[u8]);
    fn resize(&self, cols: u16, rows: u16, cell_px: (u32, u32));
    /// herdr's own scrollback; a raw stream scrolls in its emulator.
    fn scroll(&self, _up: bool, _lines: u16, _at: Option<(u16, u16)>) {}
    /// `action` is `down` / `up` / `drag` / `move`, `button` is `left` / `right` / `middle` (herdr's CLI names). A raw
    /// stream takes the mouse as input, as a terminal encodes it.
    fn mouse(&self, _action: &str, _button: &str, _column: u16, _row: u16, _modifiers: u8) {}
    /// The stream is the program's own output, not a screen rendered for this client: the emulator has the modes and
    /// history, and the mouse and wheel go to the program as a terminal encodes them.
    fn raw(&self) -> bool {
        true
    }
    /// Whether this terminal tells when its app captures the mouse.
    fn knows_mouse_capture(&self) -> bool {
        true
    }
}

pub type Attached = (Box<dyn Terminal>, UnboundedReceiver<SessionEvent>);

/// Terminals and their layout: workspaces, tabs and panes, each with its id.
pub trait Mux: Send + Sync {
    fn kind(&self) -> Kind;
    /// The whole layout, with what each pane reports of itself ([`Pane::report`]); [`Backend::snapshot`] reads that.
    fn snapshot(&self) -> Reply<'_, Snapshot>;
    /// Says when [`Self::snapshot`] may be stale, until the receiver goes; it ends when the connection does. `agents`:
    /// the panes running one, which herdr follows pane by pane.
    fn changes(&self, agents: Vec<String>) -> Reply<'_, mpsc::UnboundedReceiver<Change>>;
    fn run(&self, command: Command) -> Reply<'_, Made>;
    /// Where the pane's foreground program is now.
    fn cwd(&self, pane: String) -> Reply<'_, Option<String>>;
    /// The pane's last `lines` of text, with its colors if `ansi`.
    fn read(&self, pane: String, lines: usize, ansi: bool) -> Reply<'_, String>;
    /// The program in front of the pane's shell, if any.
    fn program(&self, pane: String) -> Reply<'_, Option<String>>;
    /// Types `text` if any, then Enter, without attaching to the pane.
    fn send(&self, pane: String, text: Option<String>) -> Reply<'_, ()>;
    /// Until a pane just made has its prompt: a shell drops typeahead before it is drawn.
    // ponytail: a fixed wait, which only the editor's split asks for
    fn prompt(&self, _pane: String) -> Reply<'_, ()> {
        Box::pin(async {
            tokio::time::sleep(Duration::from_millis(400)).await;
            Ok(())
        })
    }
    /// Attaches to `pane`: blocks, so run it off the main thread. `cell_px` is a cell in device pixels ((0, 0) while
    /// unknown).
    fn attach(&self, pane: &str, cols: u16, rows: u16, cell_px: (u32, u32), mode: Mode) -> anyhow::Result<Attached>;
    /// Keeps the checkout a workspace is on, which [`Self::snapshot`] gives back as [`crate::model::Workspace::worktree`].
    fn remember(&self, workspace: String, tree: Worktree) -> Reply<'_, ()>;
    /// The room a tab has, in cells, for a backend that sizes a tab rather than its panes.
    fn fit(&self, _tab: &str, _cols: u16, _rows: u16) {}
    /// herdr's API, where this is herdr.
    fn herdr(&self) -> Option<&Client> {
        None
    }
}

/// A repo's linked checkouts, each open as its own workspace.
pub trait Worktrees: Send + Sync {
    /// Every worktree of the workspace's repo, with the workspace each is open in. A folder outside git is refused
    /// with `not_git_worktree`.
    fn list(&self, workspace: String) -> Reply<'_, Vec<WorktreeEntry>>;
    /// Checks `branch` out as a new worktree of the workspace's repo, opened as its own workspace.
    fn create(&self, workspace: String, branch: String) -> Reply<'_, Made>;
    /// Opens the worktree at `path` of the workspace's repo as its own workspace.
    fn open(&self, workspace: String, path: String) -> Reply<'_, Made>;
    /// Deletes a linked worktree's checkout and closes its workspace; the branch stays. Without `force` a checkout
    /// with changes is refused with `dirty_worktree_requires_force`.
    fn remove(&self, workspace: String, force: bool) -> Reply<'_, ()>;
}

/// The agents running in panes.
pub trait Agents: Send + Sync {
    /// Reads `snapshot`'s agents and their statuses, up to its tabs and workspaces.
    fn read(&self, snapshot: &mut Snapshot);
    /// You looked at `pane`'s finished agent: done goes to idle.
    fn seen(&self, pane: String) -> Reply<'_, ()>;
}

/// One backend, with its worktrees and agents.
#[derive(Clone)]
pub struct Backend {
    pub mux: Arc<dyn Mux>,
    pub worktrees: Arc<dyn Worktrees>,
    pub agents: Arc<dyn Agents>,
}

impl Backend {
    pub fn on(client: Client) -> Self {
        Self { mux: Arc::new(client.clone()), worktrees: Arc::new(client.clone()), agents: Arc::new(client) }
    }

    /// A multiplexer that knows no worktrees and no agents, as all but herdr.
    fn own(mux: Arc<dyn Mux>) -> Self {
        Self { worktrees: Arc::new(Git(mux.clone())), agents: Arc::new(Own::new(mux.clone())), mux }
    }

    /// herdr on `socket` (`None`: its default session), or another backend, which has only the one place. Connects when
    /// first asked.
    pub fn of(kind: Kind, socket: Option<String>) -> Self {
        match kind {
            Kind::Herdr => Self::on(socket.filter(|p| !p.trim().is_empty()).map_or_else(Client::local, |p| Client::at(p.into()))),
            Kind::Hc => Self::own(Arc::new(crate::hc::Hc::default())),
            Kind::Rex => Self::own(Arc::new(crate::rex::Rex::default())),
            Kind::Tmux => Self::own(Arc::new(crate::tmux::Tmux::default())),
        }
    }

    pub fn kind(&self) -> Kind {
        self.mux.kind()
    }

    pub fn herdr(&self) -> Option<Client> {
        self.mux.herdr().cloned()
    }

    /// The layout with its agents.
    pub async fn snapshot(&self) -> anyhow::Result<Snapshot> {
        let mut snapshot = self.mux.snapshot().await?;
        self.agents.read(&mut snapshot);
        Ok(snapshot)
    }
}

/// The terminal's colors, which a backend answers a program's queries with.
static COLORS: Mutex<Option<(String, String)>> = Mutex::new(None);

/// The foreground and background, as `#rrggbb`.
pub fn set_colors(foreground: String, background: String) {
    *COLORS.lock().unwrap() = Some((foreground, background));
}

pub fn colors() -> Option<(String, String)> {
    COLORS.lock().unwrap().clone()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn a_new_tab_is_read_from_the_answer() {
        let result = json!({"type": "tab_created", "tab": {"tab_id": "w1:t3", "label": "3"}, "root_pane": {"pane_id": "w1:p9"}});
        assert_eq!(Made::of(&result).tab_id.as_deref(), Some("w1:t3"));
        assert_eq!(Made::of(&result).pane_id.as_deref(), Some("w1:p9"));
        assert_eq!(Made::of(&json!({"type": "pong"})), Made::default());
        // workspace.create (herdr 0.9.3) carries the new workspace's first tab the same way.
        let ws = json!({"type": "workspace_created", "workspace": {"workspace_id": "wAA", "active_tab_id": "wAA:t1"},
                        "tab": {"tab_id": "wAA:t1"}, "root_pane": {"pane_id": "wAA:p1"}});
        assert_eq!(Made::of(&ws).tab_id.as_deref(), Some("wAA:t1"));
        assert_eq!(Made::of(&ws).workspace_id.as_deref(), Some("wAA"));
    }

    #[test]
    fn a_moved_tab_is_read_from_the_answer() {
        // herdr 0.9.3 `pane.move` into another workspace: the pane gets that workspace's ids
        let result = json!({"type": "pane_move", "move_result": {"changed": true, "previous_pane_id": "w1:p2",
                            "pane": {"pane_id": "w2:p2", "tab_id": "w2:t2", "workspace_id": "w2"}}});
        assert_eq!(Made::of(&result).tab_id.as_deref(), Some("w2:t2"));
        assert_eq!(Made::of(&json!({"type": "pane_swap"})).tab_id, None);
    }

    #[test]
    fn a_split_names_its_pane() {
        assert_eq!(Made::of(&json!({"pane": {"pane_id": "w1:p2"}})).pane_id.as_deref(), Some("w1:p2"));
    }

    #[test]
    fn a_kind_reads_as_the_settings_keep_it() {
        assert_eq!(serde_json::to_value(Kind::Tmux).unwrap(), json!("tmux"));
        assert_eq!(serde_json::from_value::<Kind>(json!("hc")).unwrap(), Kind::Hc);
        assert_eq!(serde_json::from_value::<Kind>(json!("someday")).unwrap(), Kind::Herdr);
    }
}

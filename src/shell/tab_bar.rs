//! The tab bar: Zeron's terminal-panel tabs (28 px, 8 px radius, a wash under the active one) with what runs in
//! each tab as colored favicons, one per kind of agent or program (Superlogical's tabs), sized to their labels. A
//! new tab grows in and a closed one stays mounted while it shrinks out (the `Popup` recipe); a switch, a reorder
//! and a drop take effect at once, as a tab sliding made it hard to follow which one was which.
use super::*;
use std::cell::RefCell;

/// Zeron's `motion::COLLAPSE` for a tab growing in or shrinking out.
const ENTER: Duration = Duration::from_millis(180);
const EXIT: Duration = Duration::from_millis(150);
/// How long a dropped order is shown, at most, before herdr's own catches up.
const PENDING: Duration = Duration::from_millis(1500);

pub(super) const TAB_H: f32 = 28.;
/// The tabs' top inside the bar.
pub(super) const TAB_TOP: f32 = (TITLEBAR_HEIGHT - TAB_H) / 2.;
/// Space after each tab; it collapses with a closing tab so neighbours close in smoothly.
const GAP: f32 = 2.;
const PAD_L: f32 = 6.;
const PAD_R: f32 = 4.;
const INNER_GAP: f32 = 6.;
/// The favicon tile: Rex's shape, wider than tall.
const TILE_W: f32 = 20.;
const TILE_H: f32 = 16.;
/// How far each tile of a stack sits right of the one over it, and the gap the bar's color cuts around it.
const STACK_STEP: f32 = 15.;
const STACK_RING: f32 = 1.5;
/// The most tiles a tab shows, as Rex's horizontal tabs; the rest of its panes go unmarked.
const MAX_MARKS: usize = 5;
/// How often the shown workspace's plain panes are asked what they run.
const PROGRAMS_EVERY: Duration = Duration::from_secs(2);
/// The trailing slot: the status dot, swapped in place for `×` while the tab is hovered.
const SLOT: f32 = 20.;
const ZOOM_MARK: f32 = 12.;
const MIN_W: f32 = 96.;
const MAX_W: f32 = 220.;
const FONT_SIZE: f32 = 12.5;

/// Tab-bar state that outlives a frame: the scroll, the tabs as last drawn (closing ones included), and the tab
/// being dragged with its ghost's top-left in the window (written by the ghost as it follows the pointer).
#[derive(Default)]
pub(super) struct TabBar {
    /// What each plain pane runs, by pane id; a pane at its shell's prompt has none.
    pub programs: HashMap<String, String>,
    pub scroll: gpui::ScrollHandle,
    workspace: Option<String>,
    entries: Vec<Entry>,
    drag: Rc<RefCell<Option<(String, gpui::Point<Pixels>)>>>,
    /// The tab last scrolled into view; the bar follows a newly shown tab once, then leaves the scroll to you.
    revealed: Option<String>,
    /// The tab held in a drag and the slot its gap holds among the others, as the place bar's buttons do.
    pub held: Option<(String, usize)>,
    /// The order a drop asked for, shown until herdr's is it.
    pending: Option<(Vec<String>, Instant)>,
}

/// A favicon: the mark and its tile and mark colors.
type Mark = (&'static str, (Hsla, Hsla));

/// One tab as drawn.
#[derive(Clone)]
struct Shown {
    id: String,
    /// The label as herdr has it: what a rename starts from.
    label: String,
    /// The label as drawn and its faint ranges: [`name_spans`].
    name: (String, Vec<std::ops::Range<usize>>),
    status: AgentStatus,
    icon: &'static str,
    /// The favicon's tile and mark colors.
    tint: (Hsla, Hsla),
    /// The tiles after the favicon: what else runs in the tab's panes.
    more: Vec<Mark>,
    /// One of its panes is zoomed to fill it: the other panes are hidden.
    zoomed: bool,
    /// The full width: label measured, clamped.
    width: f32,
    /// A browser tab: Shepherd's own page, not herdr's; no status, its site's icon once fetched.
    browser: bool,
    image: Option<std::sync::Arc<gpui::Image>>,
    /// A browser tab's page is loading: a spinner in place of the favicon.
    loading: bool,
}

#[derive(Clone)]
struct Entry {
    tab: Shown,
    enter: Option<Instant>,
    exit: Option<Instant>,
    /// Where it is drawn: content x.
    drawn: f32,
}

impl Entry {
    fn new(tab: Shown, enter: Option<Instant>) -> Self {
        Entry { tab, enter, exit: None, drawn: 0. }
    }

    /// How far the tab is in: 0 collapsed, 1 full width.
    fn presence(&self, now: Instant) -> f32 {
        match (self.exit, self.enter) {
            (Some(since), _) => 1. - ease_out(progress(since, EXIT, now)),
            (None, Some(since)) => ease_out(progress(since, ENTER, now)),
            (None, None) => 1.,
        }
    }

    fn moving(&self, now: Instant) -> bool {
        let running = |since: Instant, d: Duration| now.duration_since(since) < d;
        self.exit.is_some_and(|s| running(s, EXIT)) || self.enter.is_some_and(|s| running(s, ENTER))
    }
}

fn progress(since: Instant, d: Duration, now: Instant) -> f32 {
    (now.duration_since(since).as_secs_f32() / d.as_secs_f32()).min(1.)
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum Phase {
    Stay,
    New,
    Gone,
}

/// The tabs to draw: `now` in herdr's order, with each tab gone since `prev` kept right after the tab it
/// followed (or first), so it shrinks out in place.
fn merge(prev: &[&str], now: &[&str]) -> Vec<(String, Phase)> {
    let mut out: Vec<(String, Phase)> =
        now.iter().map(|id| (id.to_string(), if prev.contains(id) { Phase::Stay } else { Phase::New })).collect();
    for (i, id) in prev.iter().enumerate() {
        if now.contains(id) {
            continue;
        }
        let anchor = prev[..i].iter().rev().find_map(|p| out.iter().position(|(o, _)| o == p));
        out.insert(anchor.map_or(0, |a| a + 1), (id.to_string(), Phase::Gone));
    }
    out
}

/// The icon for what runs in a tab: the agent's mark (Zeron's harness icons), a bot for agents without one,
/// a terminal for a plain shell.
fn agent_icon(agent: Option<&str>) -> &'static str {
    match agent.map(crate::ui::command_panel::agent_icon) {
        None => "icons/prompt.svg",
        Some("icons/bot.svg") => "icons/bot-filled.svg",
        Some(mark) => mark,
    }
}

/// The favicon's tile and mark: the brand's colors (black for monochrome brands, as their app icons are), an
/// inverted tile for agents without a mark, a muted one for a shell.
fn agent_tint(agent: Option<&str>, theme: &Theme) -> (Hsla, Hsla) {
    match agent {
        None => (theme.text_muted.opacity(0.55), theme.surface),
        Some("claude") => brand(0xD97757, 0xFFFFFF),
        Some("codex") => brand(0x10A37F, 0xFFFFFF),
        Some("agy" | "gemini") => brand(0x4285F4, 0xFFFFFF),
        // cursor.com's signature orange
        Some("cursor") => brand(0xF54E00, 0xFFFFFF),
        // Cognition's Devin palette blue
        Some("devin") => brand(0x0294DE, 0xFFFFFF),
        // ponytail: pi.dev's yellow accent; pi has no published brand color
        Some("pi" | "omp") => brand(0xF1BE58, 0x1A1A1A),
        // ponytail: Qwen's logo purple, by eye
        Some("qwen") => brand(0x615CED, 0xFFFFFF),
        // monochrome brands
        Some("grok" | "opencode" | "copilot" | "droid" | "kimi" | "kilo" | "qoder" | "qodercli" | "mastra" | "mastracode") => {
            brand(0x0A0A0A, 0xFFFFFF)
        }
        Some(_) => (theme.text, theme.surface),
    }
}

fn brand(bg: u32, fg: u32) -> (Hsla, Hsla) {
    (gpui::rgb(bg).into(), gpui::rgb(fg).into())
}

/// A program's favicon, by the name of its process: its own or that of the language or tool it belongs to, with
/// Simple Icons' brand colors. `None` for a program without a mark: its tab keeps the terminal's.
fn program_mark(name: &str, theme: &Theme) -> Option<Mark> {
    const WHITE: u32 = 0xFFFFFF;
    const INK: u32 = 0x1A1A1A;
    const BLACK: u32 = 0x0A0A0A;
    let name = name.to_lowercase();
    Some(match name.as_str() {
        // editors
        "nvim" | "neovim" => ("icons/neovim-mark.svg", brand(0x57A143, WHITE)),
        "vim" | "vi" => ("icons/vim-mark.svg", brand(0x019733, WHITE)),
        "hx" | "helix" => ("icons/helix-mark.svg", brand(0x281733, WHITE)),
        "kak" | "kakoune" => ("icons/kakoune-mark.svg", brand(BLACK, WHITE)),
        // version control and forges
        "git" | "lazygit" | "gitui" | "tig" => ("icons/git-mark.svg", brand(0xF03C2E, WHITE)),
        "gh" => ("icons/github-mark.svg", brand(BLACK, WHITE)),
        "glab" => ("icons/gitlab-mark.svg", brand(0xFC6D26, WHITE)),
        // languages and their tools
        "node" | "npm" | "npx" | "pnpm" | "yarn" => ("icons/node-mark.svg", brand(0x5FA04E, WHITE)),
        "bun" | "bunx" => ("icons/bun-mark.svg", brand(BLACK, WHITE)),
        "deno" => ("icons/deno-mark.svg", brand(BLACK, WHITE)),
        "cargo" | "rustc" | "rustup" => ("icons/rust-mark.svg", brand(BLACK, WHITE)),
        "go" => ("icons/go-mark.svg", brand(0x00ADD8, WHITE)),
        "flutter" | "fvm" => ("icons/flutter-mark.svg", brand(0x02569B, WHITE)),
        "dart" => ("icons/dart-mark.svg", brand(0x0175C2, WHITE)),
        "swift" | "xcodebuild" => ("icons/swift-mark.svg", brand(0xF05138, WHITE)),
        "ruby" | "irb" | "bundle" | "rake" => ("icons/ruby-mark.svg", brand(0xCC342D, WHITE)),
        "php" | "composer" => ("icons/php-mark.svg", brand(0x777BB4, WHITE)),
        "elixir" | "iex" | "mix" => ("icons/elixir-mark.svg", brand(0x8847B9, WHITE)),
        "lua" | "luajit" => ("icons/lua-mark.svg", brand(0x000080, WHITE)),
        "zig" => ("icons/zig-mark.svg", brand(0xF7A41D, INK)),
        "java" | "javac" => ("icons/java-mark.svg", brand(BLACK, WHITE)),
        "dotnet" => ("icons/dotnet-mark.svg", brand(0x512BD4, WHITE)),
        "uv" | "uvx" => ("icons/uv-mark.svg", brand(0xDE5FE9, WHITE)),
        "pixi" => ("icons/pixi-mark.svg", brand(0xF7D038, INK)),
        "proto" => ("icons/proto-mark.svg", brand(0x6E56CF, WHITE)),
        "ipython" => ("icons/python-mark.svg", brand(0x3776AB, WHITE)),
        python if python.starts_with("python") => ("icons/python-mark.svg", brand(0x3776AB, WHITE)),
        // builds
        "cmake" => ("icons/cmake-mark.svg", brand(0x064F8C, WHITE)),
        "bazel" => ("icons/bazel-mark.svg", brand(0x43A047, WHITE)),
        "turbo" => ("icons/turbo-mark.svg", brand(0xFF1E56, WHITE)),
        "make" | "gmake" => ("icons/gnu-mark.svg", brand(0xA42E2B, WHITE)),
        "task" => ("icons/task-mark.svg", brand(0x29BEB0, WHITE)),
        "moon" => ("icons/moon-mark.svg", brand(0x6F53F3, WHITE)),
        "nextflow" => ("icons/nextflow-mark.svg", brand(0x0DC09D, WHITE)),
        // containers and infrastructure
        "docker" | "docker-compose" | "lazydocker" => ("icons/docker-mark.svg", brand(0x2496ED, WHITE)),
        "podman" => ("icons/podman-mark.svg", brand(0x892CA0, WHITE)),
        "kubectl" | "k9s" => ("icons/kubernetes-mark.svg", brand(0x326CE5, WHITE)),
        "terraform" => ("icons/terraform-mark.svg", brand(0x844FBA, WHITE)),
        "packer" => ("icons/packer-mark.svg", brand(0x02A8EF, WHITE)),
        "vault" => ("icons/vault-mark.svg", brand(0xFFEC6E, INK)),
        "consul" => ("icons/consul-mark.svg", brand(0xF24C53, WHITE)),
        "nomad" => ("icons/nomad-mark.svg", brand(0x00CA8E, INK)),
        "vagrant" => ("icons/vagrant-mark.svg", brand(0x1868F2, WHITE)),
        "nix" | "nix-shell" => ("icons/nix-mark.svg", brand(0x5277C3, WHITE)),
        "nginx" => ("icons/nginx-mark.svg", brand(0x009639, WHITE)),
        "cloudflared" => ("icons/cloudflare-mark.svg", brand(0xF38020, WHITE)),
        "tailscale" => ("icons/tailscale-mark.svg", brand(BLACK, WHITE)),
        "supabase" => ("icons/supabase-mark.svg", brand(0x3FCF8E, INK)),
        // databases
        "psql" | "pgcli" | "postgres" => ("icons/postgresql-mark.svg", brand(0x4169E1, WHITE)),
        "mysql" | "mysqld" | "mariadb" => ("icons/mysql-mark.svg", brand(0x4479A1, WHITE)),
        "sqlite3" => ("icons/sqlite-mark.svg", brand(0x003B57, WHITE)),
        "redis-cli" | "redis-server" => ("icons/redis-mark.svg", brand(0xFF4438, WHITE)),
        "mongosh" | "mongod" => ("icons/mongodb-mark.svg", brand(0x47A248, WHITE)),
        // the rest
        "ollama" => ("icons/ollama-mark.svg", brand(BLACK, WHITE)),
        "tmux" => ("icons/tmux-mark.svg", brand(0x1BB91F, WHITE)),
        "ffmpeg" | "ffprobe" => ("icons/ffmpeg-mark.svg", brand(0x007808, WHITE)),
        "rclone" => ("icons/rclone-mark.svg", brand(0x3F79AD, WHITE)),
        "yazi" => ("icons/yazi-mark.svg", brand(0xF5C518, INK)),
        "bat" => ("icons/bat-mark.svg", brand(0x31369E, WHITE)),
        "d2" => ("icons/d2-mark.svg", brand(0x0D32B2, WHITE)),
        "fx" => ("icons/fx-mark.svg", brand(BLACK, WHITE)),
        "btop" => ("icons/btop-mark.svg", brand(0x1E1E20, 0xF03030)),
        "htop" => ("icons/htop-mark.svg", brand(0x009020, WHITE)),
        // no mark of their own: a pulse, in htop's green
        "top" | "btm" | "glances" => ("icons/solar/pulse.svg", brand(0x009020, WHITE)),
        "ssh" | "mosh" | "mosh-client" => ("icons/remote-server.svg", agent_tint(None, theme)),
        _ => return None,
    })
}

/// A tab's favicons from what its panes run, as (agent, program): agents, then programs, each kind once, the
/// terminal's only when nothing else shows.
fn marks(panes: &[(Option<&str>, Option<&str>)], theme: &Theme) -> Vec<Mark> {
    let agents = panes.iter().filter_map(|p| p.0).map(|a| (agent_icon(Some(a)), agent_tint(Some(a), theme)));
    let programs = panes.iter().filter(|p| p.0.is_none()).filter_map(|p| program_mark(p.1?, theme));
    let mut out: Vec<Mark> = Vec::new();
    for mark in agents.chain(programs) {
        if out.len() < MAX_MARKS && !out.iter().any(|m| m.0 == mark.0) {
            out.push(mark);
        }
    }
    if out.is_empty() {
        out.push((agent_icon(None), agent_tint(None, theme)));
    }
    out
}

/// The program a pane runs, from herdr's `pane.process_info`: the leader of its foreground group, `None` when that
/// is the shell itself.
// ponytail: a script shows as its interpreter (`brew` as ruby, `pnpm` as node); its `argv[1]` can name it if that matters.
fn foreground(info: &serde_json::Value) -> Option<String> {
    let group = info["foreground_process_group_id"].as_u64()?;
    if info["shell_pid"].as_u64() == Some(group) {
        return None;
    }
    let leader = info["foreground_processes"].as_array()?.iter().find(|p| p["pid"].as_u64() == Some(group))?;
    Some(leader["name"].as_str()?.to_string())
}

/// Keeps [`TabBar::programs`] current for the shown workspace. herdr's snapshot and events don't say when a pane's
/// foreground program changes, so this asks, also while another app has the focus: the window may still be in view.
pub(super) async fn watch_programs(this: gpui::WeakEntity<Shell>, rt: tokio::runtime::Handle, client: Client, cx: &mut gpui::AsyncApp) {
    loop {
        cx.background_executor().timer(PROGRAMS_EVERY).await;
        let Ok(panes) = this.update(cx, |s, _| s.plain_panes()) else { return };
        let client = client.clone();
        let found = rt
            .spawn(async move {
                let mut found = Vec::new();
                for pane_id in panes {
                    if let Ok(r) = client.call("pane.process_info", json!({"pane_id": pane_id})).await {
                        found.push((pane_id, foreground(&r["process_info"])));
                    }
                }
                found
            })
            .await
            .expect("tokio");
        let alive = this.update(cx, |s, cx| {
            let mut changed = false;
            for (pane_id, program) in found {
                changed |= match program {
                    Some(program) => s.tab_bar.programs.insert(pane_id, program.clone()) != Some(program),
                    None => s.tab_bar.programs.remove(&pane_id).is_some(),
                };
            }
            if changed {
                cx.notify();
            }
        });
        if alive.is_err() {
            return;
        }
    }
}

/// A label to draw and its faint ranges. `herdr.auto-title` writes `1 · branch › claude › task`: the task stays in
/// the tab's color, the number and the branch after it go faint, and the agents' own segments go, since the
/// favicon says them. A label without those comes back whole.
fn name_spans(label: &str, agents: &[&str]) -> (String, Vec<std::ops::Range<usize>>) {
    let (number, rest) = match label.split_once(" · ") {
        Some((n, rest)) if n.chars().all(|c| c.is_ascii_digit()) => (Some(n), rest),
        _ => (None, label),
    };
    let mut parts: Vec<&str> = rest.split(" › ").filter(|p| !agents.contains(p)).collect();
    let Some(name) = parts.pop() else { return (label.to_string(), Vec::new()) };
    let mut text = number.map_or_else(String::new, |n| format!("{n} · "));
    let mut faint = Vec::new();
    if !text.is_empty() {
        faint.push(0..text.len());
    }
    text.push_str(name);
    if !parts.is_empty() {
        let from = text.len();
        text = format!("{text}  {}", parts.join(" › "));
        faint.push(from..text.len());
    }
    (text, faint)
}

/// A tab's name, its faint spans drawn so.
fn name_text(t: &Shown, theme: &Theme) -> gpui::StyledText {
    let faint = gpui::HighlightStyle { color: Some(theme.text_faint), ..Default::default() };
    gpui::StyledText::new(t.name.0.clone()).with_highlights(t.name.1.iter().map(|r| (r.clone(), faint)).collect::<Vec<_>>())
}

/// The scroll (px scrolled from the left) that brings the slot `x..x + w` into a `view`-wide bar, with a
/// fade's width to spare; `None` when it already shows.
fn reveal(scrolled: f32, view: f32, x: f32, w: f32) -> Option<f32> {
    const SPARE: f32 = 32.;
    if x < scrolled {
        Some((x - SPARE).max(0.))
    } else if x + w > scrolled + view {
        Some(x + w - view + SPARE)
    } else {
        None
    }
}

/// `more` tiles after the favicon widen the tab past [`MAX_W`], so they don't take the label's room.
fn tab_width(label_w: f32, zoomed: bool, more: usize) -> f32 {
    let mark = if zoomed { ZOOM_MARK + INNER_GAP } else { 0. };
    let tiles = more as f32 * STACK_STEP;
    (PAD_L + TILE_W + tiles + INNER_GAP + label_w + mark + INNER_GAP + SLOT + PAD_R).clamp(MIN_W, MAX_W + tiles).ceil()
}

/// After the label of a tab with a zoomed pane, the zoom action's icon.
fn zoom_mark(theme: &Theme) -> gpui::Svg {
    gpui::svg().path("icons/expand-arrows.svg").size(px(ZOOM_MARK)).flex_none().text_color(theme.text_muted)
}

impl TabBar {
    /// Takes this frame's tabs; another workspace's tabs replace the old ones without motion.
    fn sync(&mut self, workspace: Option<&str>, tabs: Vec<Shown>, now: Instant) {
        if self.workspace.as_deref() != workspace {
            self.workspace = workspace.map(str::to_string);
            self.entries = tabs.into_iter().map(|tab| Entry::new(tab, None)).collect();
            return;
        }
        let prev: Vec<&str> = self.entries.iter().map(|e| e.tab.id.as_str()).collect();
        let ids: Vec<&str> = tabs.iter().map(|t| t.id.as_str()).collect();
        let order = merge(&prev, &ids);
        let entries = order
            .into_iter()
            .filter_map(|(id, phase)| {
                let old = self.entries.iter().find(|e| e.tab.id == id);
                let fresh = || tabs.iter().find(|t| t.id == id).cloned();
                match phase {
                    Phase::New => Some(Entry::new(fresh()?, Some(now))),
                    Phase::Stay => Some(Entry { tab: fresh()?, exit: None, ..old?.clone() }),
                    Phase::Gone => {
                        let old = old?;
                        let exit = old.exit.unwrap_or(now);
                        (now.duration_since(exit) < EXIT).then(|| Entry { exit: Some(exit), ..old.clone() })
                    }
                }
            })
            .collect();
        self.entries = entries;
    }

    /// Each entry's slot x (where the row lays it out), full width and presence, and records where each is drawn:
    /// in the order shown, which is the list's, or with the tab `held` (its id, and its middle's x) in the slot its
    /// middle has reached, the others closing up around its gap, or a dropped order until the list's is it. True
    /// when the held tab took another slot.
    fn layout(&mut self, now: Instant, held: Option<(&str, f32)>) -> (Vec<(f32, f32, f32)>, bool) {
        let ks: Vec<f32> = self.entries.iter().map(|e| e.presence(now)).collect();
        let step = |i: usize| (self.entries[i].tab.width + GAP) * ks[i];
        let live: Vec<usize> = (0..self.entries.len()).filter(|i| self.entries[*i].exit.is_none()).collect();
        let list: Vec<String> = live.iter().map(|i| self.entries[*i].tab.id.clone()).collect();
        let mut slid = false;
        let shown: Vec<usize> = match held.and_then(|(id, mid)| Some((live.iter().position(|i| self.entries[*i].tab.id == id)?, mid))) {
            Some((from, mid)) => {
                let mut others = live.clone();
                let h = others.remove(from);
                let id = self.entries[h].tab.id.clone();
                // the others as drawn around its gap: one after the gap goes before it once the held tab's right edge
                // passes that one's middle, one before goes after once its left edge does
                let gap = self.held.as_ref().filter(|(was, _)| *was == id).map_or(from, |(_, at)| *at);
                let half = self.entries[h].tab.width / 2.;
                let (mut x, mut slot) = (0., 0);
                for (n, i) in others.iter().enumerate() {
                    if n == gap {
                        x += step(h);
                    }
                    let middle = x + self.entries[*i].tab.width / 2.;
                    slot += usize::from(if n < gap { mid - half >= middle } else { mid + half > middle });
                    x += step(*i);
                }
                slid = self.held.as_ref().is_some_and(|(was, at)| *was == id && *at != slot);
                self.held = Some((id, slot));
                others.insert(slot, h);
                others
            }
            None => {
                self.held = None;
                let asked = self.pending.as_ref().filter(|(order, at)| {
                    let (mut a, mut b) = (order.clone(), list.clone());
                    a.sort();
                    b.sort();
                    a == b && *order != list && now.saturating_duration_since(*at) < PENDING
                });
                match asked {
                    Some((order, _)) => order.iter().filter_map(|id| live.iter().copied().find(|i| self.entries[*i].tab.id == *id)).collect(),
                    None => {
                        self.pending = None;
                        live.clone()
                    }
                }
            }
        };
        // closing tabs keep their places in the row
        let mut order = shown.clone();
        for i in (0..self.entries.len()).filter(|i| self.entries[*i].exit.is_some()) {
            order.insert(i.min(order.len()), i);
        }
        let mut targets = vec![0.; self.entries.len()];
        let mut x = 0.;
        for i in order {
            targets[i] = x;
            x += step(i);
        }
        let mut natural = 0.;
        let layout = self
            .entries
            .iter_mut()
            .enumerate()
            .map(|(i, e)| {
                let at = natural;
                natural += (e.tab.width + GAP) * ks[i];
                e.drawn = targets[i];
                (at, e.tab.width, ks[i])
            })
            .collect();
        (layout, slid)
    }

    /// A drop let go of with the held tab in its slot: that order is shown until herdr's is it. The move it asks
    /// for, `(from, to)` among the tabs not closing.
    fn drop_held(&mut self, now: Instant) -> Option<(usize, usize)> {
        let (id, slot) = self.held.clone()?;
        let mut order: Vec<String> = self.entries.iter().filter(|e| e.exit.is_none()).map(|e| e.tab.id.clone()).collect();
        let from = order.iter().position(|t| *t == id)?;
        let tab = order.remove(from);
        order.insert(slot.min(order.len()), tab);
        (from != slot).then(|| {
            self.pending = Some((order, now));
            (from, slot)
        })
    }
}

/// The favicon: the mark on a filled tile of its color, the same size for every kind; with more marks, a stack.
fn favicon(t: &Shown, ring: Hsla) -> Div {
    if t.loading {
        return div().w(px(TILE_W)).h(px(TILE_H)).flex_none().flex().items_center().justify_center().child(crate::ui::loader::glyph_spinner(3., [t.tint.1; 3]));
    }
    match &t.image {
        Some(image) => div().w(px(TILE_W)).h(px(TILE_H)).flex_none().flex().items_center().justify_center().child(gpui::img(image.clone()).size(px(16.)).rounded(px(3.))),
        None if t.more.is_empty() => tile(t.icon, t.tint),
        None => stack(t, ring),
    }
}

/// A tab's tiles overlapped, the favicon on top, each in a `ring` of what's behind them.
fn stack(t: &Shown, ring: Hsla) -> Div {
    let marks: Vec<Mark> = std::iter::once((t.icon, t.tint)).chain(t.more.iter().copied()).collect();
    div().relative().flex_none().w(px(TILE_W + t.more.len() as f32 * STACK_STEP)).h(px(TILE_H)).children(marks.into_iter().enumerate().rev().map(|(i, (icon, tint))| {
        div().absolute().left(px(i as f32 * STACK_STEP - STACK_RING)).top(px(-STACK_RING)).p(px(STACK_RING)).rounded(px(5. + STACK_RING)).bg(ring).child(tile(icon, tint))
    }))
}

/// A tab's favicon for `agent`, for lists of agents elsewhere.
pub(crate) fn agent_favicon(agent: Option<&str>, theme: &Theme) -> Div {
    tile(agent_icon(agent), agent_tint(agent, theme))
}

fn tile(icon: &'static str, (bg, fg): (Hsla, Hsla)) -> Div {
    div()
        .w(px(TILE_W))
        .h(px(TILE_H))
        .flex_none()
        .rounded(px(5.))
        .flex()
        .items_center()
        .justify_center()
        // lit from above, as Rex's tiles are; the hairline keeps a dark tile's edge on a dark bar. A tile without a
        // color (a browser tab's) stays bare.
        .when(bg.a > 0., |el| {
            el.bg(linear_gradient(180., linear_color_stop(bg.blend(gpui::white().opacity(0.07)), 0.), linear_color_stop(bg.blend(gpui::black().opacity(0.12)), 1.)))
                .border_1()
                .border_color(gpui::white().opacity(0.12))
        })
        .child(gpui::svg().path(icon).size(px(11.)).text_color(fg))
}

/// A tab's face at full width: favicon, label, trailing slot.
fn face(theme: &Theme, t: &Shown, width: f32, selected: bool) -> Div {
    // ponytail: a hovered tab's wash and a dragged tab's card differ a shade from this; the group hover can follow if it shows
    let ring = if selected { theme.surface.blend(theme.selected) } else { theme.surface };
    div()
        .w(px(width))
        .h(px(TAB_H))
        .flex_none()
        .flex()
        .items_center()
        .gap(px(INNER_GAP))
        .pl(px(PAD_L))
        .pr(px(PAD_R))
        .rounded(px(8.))
        .font_family(theme.font_ui.clone())
        .text_size(px(FONT_SIZE))
        .text_color(if selected { theme.text } else { theme.text_muted })
        .child(favicon(t, ring))
}

/// The trailing slot: the agent's status dot; a browser tab's stays empty.
fn dot_slot(theme: &Theme, t: &Shown) -> Div {
    div().size(px(SLOT)).flex_none().flex().items_center().justify_center().when(!t.browser, |el| el.child(status_dot(t.status, theme)))
}

/// A tab being dragged along the tab bar: its own face under the pointer, lifted on a card.
#[derive(Clone)]
pub(super) struct DraggedTab {
    tab: Shown,
    theme: Theme,
    /// Where the pointer grabbed the tab, from its top-left.
    grab: gpui::Point<Pixels>,
    drag: Rc<RefCell<Option<(String, gpui::Point<Pixels>)>>>,
}

impl Render for DraggedTab {
    fn render(&mut self, window: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        // gpui draws the ghost at the pointer less the grab: remember it, the held tab's gap follows it
        *self.drag.borrow_mut() = Some((self.tab.id.clone(), window.mouse_position() - self.grab));
        let theme = &self.theme;
        face(theme, &self.tab, self.tab.width, true)
            .bg(theme.card)
            .border_1()
            .border_color(theme.border)
            .shadow_lg()
            .child(div().flex_1().min_w_0().truncate().child(name_text(&self.tab, theme)))
            .when(self.tab.zoomed, |el| el.child(zoom_mark(theme)))
            .child(dot_slot(theme, &self.tab))
    }
}

impl Shell {
    /// The shown workspace's panes without an agent: an agent's pane shows its mark.
    fn plain_panes(&self) -> Vec<String> {
        let (Some(s), Some(ws)) = (&self.snapshot, &self.tab_bar.workspace) else { return Vec::new() };
        s.panes.iter().filter(|p| p.workspace_id == *ws && p.agent.is_none()).map(|p| p.pane_id.clone()).collect()
    }

    /// The held tab let go of over the bar: it lands in the slot its gap holds, browser tabs and herdr's alike.
    fn tab_dropped(&mut self, cx: &mut Context<Self>) {
        // the tabs a drop reorders: (id, browser tab or not), closing ones left out
        let order: Vec<(String, bool)> = self.tab_bar.entries.iter().filter(|e| e.exit.is_none()).map(|e| (e.tab.id.clone(), e.tab.browser)).collect();
        if let Some((from, to)) = self.tab_bar.drop_held(Instant::now()) {
            self.drop_tab(&order, from, to, cx);
        }
        cx.notify();
    }

    /// The next or previous tab in the tab bar's order, browser tabs among herdr's.
    pub(super) fn step_tab(&mut self, forward: bool, window: &mut Window, cx: &mut Context<Self>) {
        let shown = match self.shown_browser_tab() {
            Some(t) => Some(t.id.clone()),
            None => self.shown_tab().map(|t| t.tab_id),
        };
        let tabs: Vec<(String, bool)> = self.tab_bar.entries.iter().filter(|e| e.exit.is_none()).map(|e| (e.tab.id.clone(), e.tab.browser)).collect();
        let Some(i) = tabs.iter().position(|(id, _)| Some(id) == shown.as_ref()) else { return };
        let n = tabs.len();
        let (id, browser) = tabs[if forward { (i + 1) % n } else { (i + n - 1) % n }].clone();
        match browser {
            true => self.show_web(id, window, cx),
            false => self.select_tab(Some(id)),
        }
        cx.notify();
    }

    pub(super) fn render_tab_bar(
        &mut self,
        theme: &Theme,
        shown_ws: Option<String>,
        shown_tab: Option<String>,
        sidebar_w: f32,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Div {
        let now = Instant::now();
        let font = gpui::font(theme.font_ui.clone());
        let measure = |label: &str| {
            let run = gpui::TextRun { len: label.len(), font: font.clone(), color: theme.text, background_color: None, underline: None, strikethrough: None };
            f32::from(window.text_system().shape_line(label.to_string().into(), px(FONT_SIZE), &[run], None).width)
        };
        let herdr: Vec<Shown> = match (&self.snapshot, &shown_ws) {
            (Some(s), Some(ws)) => {
                s.tabs_of(ws)
                    .into_iter()
                    .map(|t| {
                        let programs = &self.tab_bar.programs;
                        let panes: Vec<_> =
                            s.panes.iter().filter(|p| p.tab_id == t.tab_id).map(|p| (p.agent.as_deref(), programs.get(&p.pane_id).map(String::as_str))).collect();
                        let agents: Vec<&str> = panes.iter().filter_map(|p| p.0).collect();
                        let name = name_spans(&t.label, &agents);
                        let mut more = marks(&panes, theme);
                        let (icon, tint) = more.remove(0);
                        let zoomed = s.layouts.iter().any(|l| l.tab_id == t.tab_id && l.zoomed);
                        Shown {
                            id: t.tab_id.clone(),
                            label: t.label.clone(),
                            status: t.agent_status,
                            icon,
                            tint,
                            zoomed,
                            width: tab_width(measure(&name.0), zoomed, more.len()),
                            name,
                            more,
                            browser: false,
                            image: None,
                            loading: false,
                        }
                    })
                    .collect()
            }
            _ => Vec::new(),
        };
        // the browser tabs among them, each after the herdr tab it follows
        let browser: Vec<(Shown, Option<String>)> = shown_ws
            .as_deref()
            .map(|ws| self.browser_tabs_of(ws))
            .unwrap_or_default()
            .into_iter()
            .map(|(id, label, image, after, loading)| {
                let width = tab_width(measure(&label), false, 0);
                let name = (label.clone(), Vec::new());
                (Shown { id, label, name, status: AgentStatus::Idle, icon: "icons/solar/global.svg", tint: (theme.surface.opacity(0.), theme.text_muted), more: Vec::new(), zoomed: false, width, browser: true, image, loading }, after)
            })
            .collect();
        let herdr_ids: Vec<&str> = herdr.iter().map(|t| t.id.as_str()).collect();
        let anchors: Vec<(&str, Option<&str>)> = browser.iter().map(|(t, after)| (t.id.as_str(), after.as_deref())).collect();
        let tabs: Vec<Shown> = super::browser::interleave(&herdr_ids, &anchors)
            .into_iter()
            .filter_map(|(id, b)| match b {
                true => browser.iter().map(|(t, _)| t).find(|t| t.id == id).cloned(),
                false => herdr.iter().find(|t| t.id == id).cloned(),
            })
            .collect();
        self.tab_bar.sync(shown_ws.as_deref(), tabs, now);

        let bar_x = f32::from(self.tab_bar.scroll.bounds().origin.x) + f32::from(self.tab_bar.scroll.offset().x);
        let dragging = cx.has_active_drag();
        let dragged = self.tab_bar.drag.borrow().clone();
        // the drag ended, dropped on a tab or anywhere
        if !dragging {
            self.tab_bar.drag.borrow_mut().take();
        }
        // the held tab's middle, where its ghost is, in the row
        let held = dragged.as_ref().filter(|_| dragging).and_then(|(id, at)| {
            let width = self.tab_bar.entries.iter().find(|e| e.tab.id == *id)?.tab.width;
            Some((id.clone(), f32::from(at.x) - bar_x + width / 2.))
        });
        let lifted_id = dragged.filter(|_| dragging).map(|(id, _)| id);

        let (layout, slid) = self.tab_bar.layout(now, held.as_ref().map(|(id, mid)| (id.as_str(), *mid)));
        if slid {
            haptic();
        }
        // A newly shown tab scrolls into view, followed while it grows in (gpui clamps the offset to the content).
        if let Some(i) = self.tab_bar.entries.iter().position(|e| Some(&e.tab.id) == shown_tab.as_ref() && e.exit.is_none()) {
            let entry = &self.tab_bar.entries[i];
            if self.tab_bar.revealed.as_ref() != shown_tab.as_ref() || entry.enter.is_some_and(|s| now.duration_since(s) < ENTER) {
                let scroll = &self.tab_bar.scroll;
                let view = f32::from(scroll.bounds().size.width);
                if view > 0. {
                    if let Some(to) = reveal(-f32::from(scroll.offset().x), view, entry.drawn, entry.tab.width) {
                        scroll.set_offset(gpui::point(px(-to), scroll.offset().y));
                    }
                    self.tab_bar.revealed = shown_tab.clone();
                }
            }
        }
        if self.tab_bar.entries.iter().any(|e| e.moving(now)) {
            window.request_animation_frame();
        }

        let mut items = Vec::new();
        for (entry, (slot_x, full_w, k)) in self.tab_bar.entries.clone().into_iter().zip(layout) {
            let t = entry.tab.clone();
            let closing = entry.exit.is_some();
            let selected = !closing && shown_tab.as_deref() == Some(t.id.as_str());
            let editing = (!closing).then(|| self.rename_field(&RenameTarget::Tab(t.id.clone()), cx)).flatten();
            let draggable = editing.is_none();
            let full_w = if draggable { full_w } else { MAX_W };
            // held, a placeholder keeps its gap and the ghost carries the tab
            let holding = lifted_id.as_deref() == Some(t.id.as_str());
            let group = SharedString::from(format!("tab-row-{}", t.id));
            // The face keeps its full width and is clipped while the tab grows or shrinks.
            let content = face(theme, &t, full_w, selected)
                .map(|el| match editing {
                    Some(field) => el.child(field),
                    None => el.child(div().flex_1().min_w_0().truncate().child(name_text(&t, theme))),
                })
                .when(t.zoomed && draggable, |el| el.child(zoom_mark(theme)))
                .child(
                    div()
                        .id(SharedString::from(format!("tab-x-{}", t.id)))
                        .tooltip(tip("Close tab", Some("CloseTab")))
                        .size(px(SLOT))
                        .flex_none()
                        .relative()
                        .rounded(px(6.))
                        .hover(|s| s.bg(theme.hover))
                        .child(dot_slot(theme, &t).absolute().inset_0().group_hover(group.clone(), |s| s.opacity(0.)))
                        .child(
                            div()
                                .absolute()
                                .inset_0()
                                .flex()
                                .items_center()
                                .justify_center()
                                .opacity(0.)
                                .group_hover(group.clone(), |s| s.opacity(1.))
                                .child(gpui::svg().path("icons/close.svg").size(px(12.)).text_color(theme.text_muted)),
                        )
                        .when(!closing, |el| {
                            let (id, browser) = (t.id.clone(), t.browser);
                            // the tab owns a drag: claim the press so it closes instead of starting one
                            el.on_mouse_down(MouseButton::Left, |_, window, cx| {
                                window.prevent_default();
                                cx.stop_propagation();
                            })
                            .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                                cx.stop_propagation();
                                this.close_tab(&id, browser, cx);
                            }))
                        }),
                );
            let tab = div()
                .id(SharedString::from(format!("tab-{}", t.id)))
                .group(group)
                .relative()
                .left(px(entry.drawn - slot_x))
                .flex_none()
                .h(px(TAB_H))
                .w(px(full_w * k))
                .mr(px(GAP * k))
                .overflow_hidden()
                .opacity(k)
                .when(selected && !holding, |el| el.bg(theme.selected))
                .when(holding, |el| el.border_1().border_color(theme.border).bg(theme.wash(0.04)))
                .rounded(px(8.))
                .cursor_pointer()
                // off the titlebar's drag strip: a press here drags the tab, not the window; the wheel still scrolls the bar
                .block_mouse_except_scroll()
                .on_mouse_down(MouseButton::Left, |_, window, _| window.prevent_default())
                .when(!selected && !closing, |el| el.hover(|s| s.bg(theme.hover)))
                .child(content.when(holding, |el| el.opacity(0.)));
            if closing {
                items.push(tab.into_any_element());
                continue;
            }
            let id = t.id.clone();
            let dragged = draggable.then(|| DraggedTab {
                tab: t.clone(),
                theme: theme.clone(),
                grab: gpui::Point::default(),
                drag: self.tab_bar.drag.clone(),
            });
            let drop_bg = theme.accent.opacity(0.15);
            let (label, browser) = (t.label.clone(), t.browser);
            items.push(
                tab.when(!browser, |el| {
                    el.on_mouse_down(MouseButton::Right, cx.listener({
                        let id = id.clone();
                        move |this, e: &MouseDownEvent, _, cx| {
                            this.tab_menu = Some(crate::ui::Popup::new((id.clone(), e.position)));
                            cx.notify();
                        }
                    }))
                })
                // Middle-click closes, like every tab strip (Zeron §1.10).
                .on_mouse_down(MouseButton::Middle, cx.listener({
                    let id = id.clone();
                    move |this, _: &MouseDownEvent, _, cx| this.close_tab(&id, browser, cx)
                }))
                .when_some(dragged, |el, dragged| {
                    el.on_drag(dragged, |d, grab, _, cx| {
                        let d = DraggedTab { grab, ..d.clone() };
                        cx.new(|_| d)
                    })
                })
                .when(!browser, |el| pane_drag::accept_pane(el, &id, drop_bg, cx))
                // a tab's hitbox hides the bar's from the drop: each takes it too
                .on_drop(cx.listener(|this, _: &DraggedTab, _, cx| this.tab_dropped(cx)))
                .on_click(cx.listener(move |this, e: &ClickEvent, window, cx| {
                    if browser {
                        return this.show_web(id.clone(), window, cx);
                    }
                    if e.click_count() == 2 {
                        this.start_rename(RenameTarget::Tab(id.clone()), label.clone(), window, cx);
                        return;
                    }
                    this.select_tab(Some(id.clone()));
                    cx.notify();
                }))
                .into_any_element(),
            );
        }

        let scroll = &self.tab_bar.scroll;
        div()
            .h(px(TITLEBAR_HEIGHT))
            .flex_none()
            // opaque, so the edge fades (same color) blend in exactly
            .bg(theme.surface)
            .flex()
            .items_center()
            .gap(px(2.))
            // the 28 px `+` sits as far from the right edge as from the top and bottom, and clear of the window's
            // buttons while no review panel sits under them
            .pr(px(TAB_TOP + if self.review_open { 0. } else { window_controls_w(cx) }))
            // clear the traffic lights and the sidebar toggle once the sidebar no longer sits under them
            .pl(px((TITLEBAR_CLUSTER - sidebar_w).max(8.)))
            .child(
                div()
                    // off macOS the `+` follows the last tab, apart from the window's buttons at the right end
                    .when(cfg!(target_os = "macos"), |el| el.flex_1())
                    .min_w_0()
                    .h_full()
                    .relative()
                    .child(
                        div()
                            .id("tabbar")
                            // GPUI tells Windows the control area of every hitbox under the pointer, scroll-only
                            // blockers included: without blocking here a press on a tab is the drag strip's, and
                            // Windows moves the window. The bar still scrolls, being the blocker.
                            .when(cfg!(windows), |el| el.occlude())
                            .size_full()
                            .flex()
                            .items_center()
                            .overflow_x_scroll()
                            .track_scroll(scroll)
                            // the held tab's gap follows the pointer; let go of, it lands in that slot, browser tabs
                            // and herdr's alike
                            .on_drag_move::<DraggedTab>(cx.listener(|_, _, _, cx| cx.notify()))
                            .on_drop(cx.listener(|this, _: &DraggedTab, _, cx| this.tab_dropped(cx)))
                            // re-render so the edge fades follow the scroll position
                            .on_scroll_wheel(cx.listener(|_, _, _, cx| cx.notify()))
                            .children(items),
                    )
                    // ponytail: gpui has no alpha mask, so the fades are gradients of the bar's own color over the tabs
                    .map(|el| {
                        let (offset, max) = (-f32::from(scroll.offset().x), f32::from(scroll.max_offset().x));
                        let fade = |angle: f32| {
                            div().absolute().top_0().bottom_0().w(px(32.)).bg(linear_gradient(
                                angle,
                                linear_color_stop(theme.surface, 0.),
                                linear_color_stop(theme.surface.opacity(0.), 1.),
                            ))
                        };
                        el.when(offset > 0.5, |el| el.child(fade(90.).left_0()))
                            .when(offset < max - 0.5, |el| el.child(fade(270.).right_0()))
                    }),
            )
            // `+` stays in view; only the tabs scroll
            .when_some(shown_ws, |el, ws| {
                let menu_ws = ws.clone();
                el.child(
                    icon_button(theme, "new-tab", "+")
                        .tooltip(tip("New tab", Some("NewTab")))
                        .size(px(TAB_H))
                        .rounded(px(8.))
                        .text_size(px(19.))
                        .block_mouse_except_scroll()
                        .when(cfg!(windows), |el| el.occlude()) // as the tabs, off the drag strip
                        .on_mouse_down(MouseButton::Left, |_, window, _| window.prevent_default())
                        .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| this.new_tab(ws.clone(), cx))),
                )
                // the chevron: a terminal or a browser tab
                .child(
                    icon_button(theme, "new-tab-more", "")
                        .tooltip(tip("New…", None))
                        .w(px(16.))
                        .h(px(TAB_H))
                        .ml(px(-2.))
                        .rounded(px(6.))
                        .when(self.new_tab_menu.is_some(), |el| el.bg(theme.hover))
                        .child(gpui::svg().path("icons/alt-arrow-down.svg").size(px(12.)).text_color(theme.text_muted))
                        .block_mouse_except_scroll()
                        .when(cfg!(windows), |el| el.occlude())
                        .on_mouse_down(MouseButton::Left, |_, window, _| window.prevent_default())
                        .on_click(cx.listener(move |this, e: &ClickEvent, _, cx| {
                            // under the chevron, the menu's right edge at it
                            let at = gpui::point(e.position().x + px(8.), px(TITLEBAR_HEIGHT - 2.));
                            this.new_tab_menu = Some(crate::ui::Popup::new((menu_ws.clone(), at)));
                            cx.notify();
                        })),
                )
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tab(id: &str) -> Shown {
        Shown { id: id.into(), label: id.into(), name: (id.into(), Vec::new()), status: AgentStatus::Idle, icon: "", tint: (gpui::black(), gpui::white()), more: Vec::new(), zoomed: false, width: 100., browser: false, image: None, loading: false }
    }

    #[test]
    fn closed_tabs_stay_in_place_and_new_ones_join() {
        use Phase::*;
        let got = merge(&["a", "b", "c", "d"], &["a", "c", "d", "e"]);
        let want = [("a", Stay), ("b", Gone), ("c", Stay), ("d", Stay), ("e", New)];
        assert_eq!(got, want.map(|(id, p)| (id.to_string(), p)));
        // the first tab closing keeps the front; neighbours closing together keep their order
        let got = merge(&["a", "b", "c"], &["c"]);
        assert_eq!(got, [("a", Gone), ("b", Gone), ("c", Stay)].map(|(id, p)| (id.to_string(), p)));
        // a reorder is herdr's new order
        assert_eq!(merge(&["a", "b"], &["b", "a"]), [("b", Stay), ("a", Stay)].map(|(id, p)| (id.to_string(), p)));
    }

    #[test]
    fn the_shown_tab_scrolls_into_view() {
        assert_eq!(reveal(0., 300., 100., 100.), None);
        assert_eq!(reveal(0., 300., 400., 100.), Some(232.)); // off the right: its right edge plus a fade
        assert_eq!(reveal(500., 300., 100., 100.), Some(68.)); // off the left
        assert_eq!(reveal(50., 300., 10., 100.), Some(0.));
    }

    #[test]
    fn agents_get_their_mark() {
        assert_eq!(agent_icon(None), "icons/prompt.svg");
        assert_eq!(agent_icon(Some("claude")), "icons/claude-mark.svg");
        assert_eq!(agent_icon(Some("codex")), "icons/openai-mark.svg");
        assert_eq!(agent_icon(Some("kimi")), "icons/kimi-mark.svg");
        assert_eq!(agent_icon(Some("letta")), "icons/bot-filled.svg");
    }

    #[test]
    fn a_tab_shows_each_kind_once_agents_first() {
        let theme = Theme::for_appearance(gpui::WindowAppearance::Dark);
        let icons = |panes: &[(Option<&str>, Option<&str>)]| marks(panes, &theme).into_iter().map(|m| m.0).collect::<Vec<_>>();
        assert_eq!(icons(&[(None, None), (None, None)]), ["icons/prompt.svg"]);
        assert_eq!(icons(&[(None, Some("nvim")), (Some("claude"), None), (None, None), (None, Some("nvim"))]), ["icons/claude-mark.svg", "icons/neovim-mark.svg"]);
        // a program without a mark, and an agent's stale program, are nothing to show
        assert_eq!(icons(&[(None, Some("some-tool"))]), ["icons/prompt.svg"]);
        assert_eq!(icons(&[(Some("claude"), Some("nvim"))]), ["icons/claude-mark.svg"]);
        assert_eq!(icons(&[(None, Some("Python")), (None, Some("python3.14"))]), ["icons/python-mark.svg"]);
        assert_eq!(icons(&[(None, Some("btop"))]), ["icons/btop-mark.svg"]);
        assert_eq!(icons(&[(None, Some("top"))]), ["icons/solar/pulse.svg"]);
        let many: Vec<_> = ["git", "go", "cargo", "node", "docker", "tmux"].map(|p| (None, Some(p))).to_vec();
        assert_eq!(icons(&many).len(), MAX_MARKS);
    }

    #[test]
    fn a_pane_runs_its_foreground_groups_leader() {
        let info = |group: u64| json!({"shell_pid": 10, "foreground_process_group_id": group, "foreground_processes": [{"pid": 12, "name": "rg"}, {"pid": 11, "name": "nvim"}]});
        assert_eq!(foreground(&info(11)).as_deref(), Some("nvim"));
        assert_eq!(foreground(&info(10)), None); // the shell's prompt
        assert_eq!(foreground(&info(99)), None); // gone already
    }

    #[test]
    #[allow(clippy::single_range_in_vec_init)]
    fn a_label_loses_the_agent_and_fades_the_rest() {
        assert_eq!(name_spans("harness", &[]), ("harness".to_string(), vec![]));
        assert_eq!(name_spans("1 · claude › Новый GUI", &["claude"]), ("1 · Новый GUI".to_string(), vec![0..5]));
        assert_eq!(name_spans("2 · main › claude › Task", &["claude"]), ("2 · Task  main".to_string(), vec![0..5, 9..15]));
        assert_eq!(name_spans("3 · mochi › nvim", &[]), ("3 · nvim  mochi".to_string(), vec![0..5, 9..16]));
        // only the agent: nothing else to name the tab
        assert_eq!(name_spans("1 · claude", &["claude"]), ("1 · claude".to_string(), vec![]));
        assert_eq!(name_spans("v2 · draft", &[]), ("v2 · draft".to_string(), vec![]));
    }

    #[test]
    fn a_closing_tab_shrinks_out_then_goes() {
        let t0 = Instant::now();
        let mut bar = TabBar::default();
        bar.sync(Some("w"), vec![tab("a"), tab("b")], t0);
        bar.sync(Some("w"), vec![tab("b")], t0);
        assert_eq!(bar.entries.len(), 2);
        assert_eq!(bar.layout(t0, None).0[1].0, 102.); // `a` still holds its slot as it starts to close
        bar.sync(Some("w"), vec![tab("b")], t0 + EXIT);
        assert_eq!(bar.entries.len(), 1);
        // another workspace swaps the tabs without motion
        bar.sync(Some("v"), vec![tab("c")], t0 + EXIT);
        assert_eq!(bar.entries[0].presence(t0 + EXIT), 1.);
    }

    #[test]
    fn reordered_tabs_are_in_their_slots_at_once() {
        let t0 = Instant::now();
        let mut bar = TabBar::default();
        bar.sync(Some("w"), vec![tab("a"), tab("b")], t0);
        bar.layout(t0, None);
        bar.sync(Some("w"), vec![tab("b"), tab("a")], t0);
        bar.layout(t0, None);
        assert_eq!((bar.entries[0].drawn, bar.entries[1].drawn), (0., 102.));
        assert!(!bar.entries.iter().any(|e| e.moving(t0)));
    }

    #[test]
    fn a_held_tab_takes_the_slot_its_middle_reaches_and_lands_there() {
        let t0 = Instant::now();
        let mut bar = TabBar::default();
        bar.sync(Some("w"), vec![tab("a"), tab("b"), tab("c")], t0);
        bar.layout(t0, None);
        // `a` (middle 50) dragged right: its right edge (150) short of b's middle (152), it keeps its slot; past it,
        // it takes b's
        let (_, slid) = bar.layout(t0, Some(("a", 100.)));
        assert_eq!(bar.held, Some(("a".into(), 0)));
        assert!(!slid);
        let (_, slid) = bar.layout(t0, Some(("a", 110.)));
        assert_eq!(bar.held, Some(("a".into(), 1)));
        assert!(slid);
        // held there, it stays: its left edge (60) is past b's middle (50) now b has closed up
        let (_, slid) = bar.layout(t0, Some(("a", 110.)));
        assert!(!slid);
        // b is in a's place
        assert_eq!((bar.entries[0].drawn, bar.entries[1].drawn), (102., 0.));
        // let go: the move asked for, and that order kept until herdr's is it
        assert_eq!(bar.drop_held(t0), Some((0, 1)));
        bar.layout(t0, None);
        assert_eq!((bar.entries[0].drawn, bar.entries[1].drawn), (102., 0.));
        bar.sync(Some("w"), vec![tab("b"), tab("a"), tab("c")], t0);
        bar.layout(t0, None);
        assert_eq!((bar.entries[0].drawn, bar.entries[1].drawn), (0., 102.));
        assert!(bar.pending.is_none());
    }
}

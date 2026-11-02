//! The tab bar: herdr's tabs, with what runs in each as colored favicons, one per kind of agent or program
//! (Superlogical's tabs). The strip itself is `tabs`'.
use super::tabs::{Leading, Mark, TAB_TOP, Tab, Tabs, label_width, new_tab_button, plain_tint, tab_element, tab_width, tile};
use super::*;

/// The most tiles a tab shows, as Rex's horizontal tabs; the rest of its panes go unmarked.
const MAX_MARKS: usize = 5;
/// How often the shown workspace's plain panes are asked what they run.
const PROGRAMS_EVERY: Duration = Duration::from_secs(2);

#[derive(Default)]
pub(super) struct TabBar {
    /// What each plain pane runs, by pane id; a pane at its shell's prompt has none.
    pub programs: HashMap<String, String>,
    pub tabs: Tabs,
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
        None => plain_tint(theme),
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
/// Keeps [`TabBar::programs`] current for the shown workspace. herdr's snapshot and events don't say when a pane's
/// foreground program changes, so this asks, also while another app has the focus: the window may still be in view.
pub(super) async fn watch_programs(this: gpui::WeakEntity<Shell>, rt: tokio::runtime::Handle, backend: Backend, cx: &mut gpui::AsyncApp) {
    loop {
        cx.background_executor().timer(PROGRAMS_EVERY).await;
        let Ok(panes) = this.update(cx, |s, _| s.plain_panes()) else { return };
        let backend = backend.clone();
        let found = rt
            .spawn(async move {
                let mut found = Vec::new();
                for pane_id in panes {
                    if let Ok(program) = backend.mux.program(pane_id.clone()).await {
                        found.push((pane_id, program));
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

/// A tab's favicon for `agent`, for lists of agents elsewhere.
pub(crate) fn agent_favicon(agent: Option<&str>, theme: &Theme) -> Div {
    tile(agent_icon(agent), agent_tint(agent, theme))
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

impl Shell {
    /// The shown workspace's panes without an agent: an agent's pane shows its mark.
    fn plain_panes(&self) -> Vec<String> {
        let (Some(s), Some(ws)) = (&self.snapshot, &self.tab_bar.tabs.scope) else { return Vec::new() };
        s.panes.iter().filter(|p| p.workspace_id == *ws && p.agent.is_none()).map(|p| p.pane_id.clone()).collect()
    }

    /// The held tab let go of over the bar: it lands in the slot its gap holds.
    fn tab_dropped(&mut self, cx: &mut Context<Self>) {
        // the tabs a drop reorders, closing ones left out
        let order: Vec<String> = self.tab_bar.tabs.entries.iter().filter(|e| e.exit.is_none()).map(|e| e.tab.id.clone()).collect();
        if let Some((from, to)) = self.tab_bar.tabs.drop_held(Instant::now())
            && let (Some(tab_id), Some(insert_index)) = (order.get(from).cloned(), insert_index(from, to))
        {
            self.send(Command::MoveTab { tab_id, insert_index }, cx, |_, _, _| {});
        }
        cx.notify();
    }

    /// The next or previous tab: the panel's with the keyboard in it, else the tab bar's.
    pub(super) fn step_tab(&mut self, forward: bool, cx: &mut Context<Self>) {
        if self.panel_focused() {
            return self.update_panel(cx, |p| p.step(forward));
        }
        let shown = self.shown_tab().map(|t| t.tab_id);
        let tabs: Vec<String> = self.tab_bar.tabs.entries.iter().filter(|e| e.exit.is_none()).map(|e| e.tab.id.clone()).collect();
        let Some(i) = tabs.iter().position(|id| Some(id) == shown.as_ref()) else { return };
        let n = tabs.len();
        self.select_tab(Some(tabs[if forward { (i + 1) % n } else { (i + n - 1) % n }].clone()));
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
        let tabs: Vec<Tab> = match (&self.snapshot, &shown_ws) {
            (Some(s), Some(ws)) => {
                s.tabs_of(ws)
                    .into_iter()
                    .map(|t| {
                        let programs = &self.tab_bar.programs;
                        let panes: Vec<_> =
                            s.panes.iter().filter(|p| p.tab_id == t.tab_id).map(|p| (p.agent.as_deref(), programs.get(&p.pane_id).map(String::as_str))).collect();
                        let agents: Vec<&str> = panes.iter().filter_map(|p| p.0).collect();
                        let name = name_spans(&t.label, &agents);
                        let marks = marks(&panes, theme);
                        let zoomed = s.layouts.iter().any(|l| l.tab_id == t.tab_id && l.zoomed);
                        Tab {
                            id: t.tab_id.clone(),
                            label: t.label.clone(),
                            status: Some(t.agent_status),
                            zoomed,
                            width: tab_width(label_width(&name.0, theme, window), zoomed, marks.len() - 1),
                            name,
                            leading: Leading::Marks(marks),
                        }
                    })
                    .collect()
            }
            _ => Vec::new(),
        };
        let placed = self.tab_bar.tabs.frame(shown_ws.as_deref(), tabs, shown_tab.as_deref(), window, cx);

        let mut items = Vec::new();
        for p in &placed {
            let t = &p.tab;
            let editing = (!p.closing).then(|| self.rename_field(&RenameTarget::Tab(t.id.clone()), cx)).flatten().map(IntoElement::into_any_element);
            let draggable = editing.is_none();
            let id = t.id.clone();
            let close = id.clone();
            let tab = tab_element(p, theme, editing, Some("CloseTab"), move |this, cx| this.send(Command::CloseTab { tab_id: close.clone() }, cx, |_, _, _| {}), window, cx);
            if p.closing {
                items.push(tab.into_any_element());
                continue;
            }
            let drop_bg = theme.accent.opacity(0.15);
            let label = t.label.clone();
            let tab = if draggable { self.tab_bar.tabs.liftable(tab, p, theme, Shell::tab_dropped, cx) } else { tab };
            items.push(
                tab.on_mouse_down(MouseButton::Right, cx.listener({
                    let id = id.clone();
                    move |this, e: &MouseDownEvent, _, cx| {
                        this.tab_menu = Some(crate::ui::Popup::new((id.clone(), e.position)));
                        cx.notify();
                    }
                }))
                .map(|el| pane_drag::accept_pane(el, &id, drop_bg, cx))
                .on_click(cx.listener(move |this, e: &ClickEvent, window, cx| {
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

        let panel_shown = self.right_panel_shown(window);
        // no room for the panel, no button to open it
        let room = crate::right_panel::fits(f32::from(window.viewport_size().width), sidebar_w);
        let toggle = shown_ws.is_some() && !panel_shown && room;
        let row = self.tabs_row(&self.tab_bar.tabs, "tabbar", theme, items, Shell::tab_dropped, cx);
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
            .pr(px(TAB_TOP + if panel_shown { 0. } else { window_controls_w(cx) }))
            // clear the traffic lights and the sidebar toggle once the sidebar no longer sits under them
            .pl(px((TITLEBAR_CLUSTER - sidebar_w).max(8.)))
            // off macOS the `+` follows the last tab, apart from the window's buttons at the right end
            .child(row.when(cfg!(target_os = "macos"), |el| el.flex_1()))
            .when_some(shown_ws, |el, ws| {
                el.child(
                    new_tab_button(theme, "new-tab")
                        .tooltip(tip("New tab", Some("NewTab")))
                        .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| this.new_tab(ws.clone(), cx))),
                )
            })
            // the toggle, the panel's own while it's open
            .when(toggle, |el| {
                el.child(
                    titlebar_button_with(theme, "right-panel-toggle", crate::ui::sidebar_glyph(self.right_panel_slide(window), true, 16., theme.text_muted))
                        .ml_auto()
                        .flex_none()
                        .tooltip(tip("Toggle right panel", Some("ToggleRightPanel")))
                        .on_click(cx.listener(|this, _: &ClickEvent, window, cx| this.toggle_right_panel(window, cx))),
                )
            })
    }
}

/// herdr's insert index for the tab at `from` dropped on the one at `to`, which counts it still in place; none when
/// it stays.
fn insert_index(from: usize, to: usize) -> Option<usize> {
    (from != to).then_some(if to > from { to + 1 } else { to })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_dropped_tab_goes_to_herdr_counted_still_in_place() {
        assert_eq!(insert_index(0, 2), Some(3)); // moving right
        assert_eq!(insert_index(3, 1), Some(1)); // moving left
        assert_eq!(insert_index(2, 2), None);
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
}

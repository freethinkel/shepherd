//! PoC (`SHEPHERD_CLIENT_SHELL=1`): the window as one herdr client shell. herdr shows it the tab Shepherd shows,
//! sized to the pane grid, and its panes take their cut of the shell's surface (see `crate::herdr::client_shell`).
use super::*;
use crate::herdr::client_shell::{ClientShell, Event, HostThemeUpdate as Update, Layout};
use crate::herdr::popup::Event as PopupEvent;
use crate::model::Frac;
use crate::terminal::pane::ShellConnection;
use futures::StreamExt;
use std::sync::Arc;

/// The connection and what it was last told, so a render tells herdr only what changed.
pub(super) struct ShellView {
    conn: Arc<ClientShell>,
    layout: Layout,
    tab: Option<String>,
    size: Option<(u16, u16)>,
    theme: Option<Hsla>,
    active: Option<bool>,
}

/// Connects, hands the panes over to the shell, and follows it; reconnects while the window lives.
pub(super) async fn shell_loop(this: gpui::WeakEntity<Shell>, cx: &mut gpui::AsyncApp) {
    loop {
        let Ok((cols, rows)) = this.update(cx, |s, cx| s.grid_cells(cx).unwrap_or((120, 40))) else { return };
        let connect = cx.background_executor().spawn(async move { ClientShell::connect(cols, rows) });
        if let Ok((conn, mut rx)) = connect.await {
            let started = this.update(cx, |s, cx| {
                cx.set_global(ShellConnection(conn.clone()));
                s.client_shell = Some(ShellView { conn, layout: Layout::default(), tab: None, size: None, theme: None, active: None });
                // the panes shown so far attached on their own: over to the shell
                for pane in s.panes.values() {
                    pane.update(cx, |p, cx| p.reattach(false, cx));
                }
            });
            if started.is_err() {
                return;
            }
            while let Some(event) = rx.next().await {
                let alive = this.update(cx, |s, cx| {
                    match event {
                        Event::Layout(layout) => {
                            if let Some(view) = &mut s.client_shell {
                                view.layout = layout;
                            }
                        }
                        Event::Popup(PopupEvent::Popup(surface)) => s.show_popup(surface, cx),
                        Event::Popup(PopupEvent::Commands(commands)) => s.herdr_commands = commands,
                    }
                    cx.notify();
                });
                if alive.is_err() {
                    return;
                }
            }
        }
        let gone = this.update(cx, |s, cx| {
            s.client_shell = None;
            if cx.has_global::<ShellConnection>() {
                cx.remove_global::<ShellConnection>();
            }
            s.show_popup(None, cx);
        });
        if gone.is_err() {
            return;
        }
        // ponytail: retry every 5 s, as the popup watch does
        cx.background_executor().timer(Duration::from_secs(5)).await;
    }
}

impl Shell {
    /// Tells herdr what changed since the last render: the shown tab, the grid's size, the theme, the window's focus.
    pub(super) fn sync_client_shell(&mut self, tab: Option<&str>, window: &Window, cx: &App) {
        let cells = self.grid_cells(cx);
        let Some(view) = &mut self.client_shell else { return };
        if let Some(tab) = tab.filter(|t| view.tab.as_deref() != Some(*t)) {
            view.conn.focus_tab(tab);
            view.tab = Some(tab.into());
        }
        if let Some((cols, rows)) = cells.filter(|c| view.size != Some(*c)) {
            view.conn.resize(cols, rows);
            view.size = Some((cols, rows));
        }
        let theme = Theme::of(cx);
        if view.theme != Some(theme.terminal.background) {
            view.conn.theme(host_theme(theme));
            view.theme = Some(theme.terminal.background);
        }
        let active = window.is_window_active();
        if view.active != Some(active) {
            view.conn.focused(active);
            view.active = Some(active);
        }
    }

    /// The shown tab's panes where herdr laid them out for this shell, once its surface is of that tab.
    pub(super) fn client_shell_frames(&self, snapshot_frames: &[PaneFrame]) -> Option<Vec<PaneFrame>> {
        let layout = &self.client_shell.as_ref()?.layout;
        let ours = !layout.panes.is_empty() && layout.panes.iter().all(|p| snapshot_frames.iter().any(|f| f.pane_id == p.pane_id));
        if !ours || layout.cols == 0 || layout.rows == 0 {
            return None;
        }
        let (cols, rows) = (layout.cols as f32, layout.rows as f32);
        Some(
            layout
                .panes
                .iter()
                .map(|p| {
                    let (x, y, w, h) = p.rect;
                    let frame = Frac { x: x as f32 / cols, y: y as f32 / rows, w: w as f32 / cols, h: h as f32 / rows };
                    PaneFrame { pane_id: p.pane_id.clone(), frame, focused: p.focused }
                })
                .collect(),
        )
    }
}

fn host_theme(theme: &Theme) -> Vec<Update> {
    use crate::herdr::client_shell::{DefaultColorKind as Kind, HostAppearance as Appearance, HostColor as Color};
    let rgb = |c: Hsla| {
        let c = gpui::Rgba::from(c);
        let b = |v: f32| (v.clamp(0.0, 1.0) * 255.0).round() as u8;
        Color { r: b(c.r), g: b(c.g), b: b(c.b) }
    };
    vec![
        Update::DefaultColor { kind: Kind::Foreground, color: rgb(theme.terminal.foreground) },
        Update::DefaultColor { kind: Kind::Background, color: rgb(theme.terminal.background) },
        Update::PaletteColors(theme.terminal.ansi.iter().enumerate().map(|(i, c)| (i as u8, rgb(*c))).collect()),
        Update::Appearance(if theme.appearance == crate::theme::Appearance::Dark { Appearance::Dark } else { Appearance::Light }),
    ]
}

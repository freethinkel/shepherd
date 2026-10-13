//! PoC (`SHEPHERD_CLIENT_SHELL=1`): the window's one herdr client shell, next to its panes' attaches. It carries
//! Shepherd's theme to herdr, herdr's popup and `[[keys.command]]` to Shepherd (see `crate::herdr::client_shell`).
use super::*;
use crate::herdr::client_shell::{ClientShell, HostThemeUpdate as Update};
use crate::herdr::popup::Event as PopupEvent;
use crate::terminal::pane::ShellConnection;
use futures::StreamExt;
use std::sync::Arc;

/// The connection and what it was last told, so a render tells herdr only what changed.
pub(super) struct ShellView {
    conn: Arc<ClientShell>,
    size: Option<(u16, u16)>,
    theme: Option<Hsla>,
    active: bool,
}

/// Connects and follows the shell; reconnects while the window lives.
pub(super) async fn shell_loop(this: gpui::WeakEntity<Shell>, cx: &mut gpui::AsyncApp) {
    loop {
        let Ok((cols, rows)) = this.update(cx, |s, cx| s.grid_cells(cx).unwrap_or((120, 40))) else { return };
        let connect = cx.background_executor().spawn(async move { ClientShell::connect(cols, rows) });
        if let Ok((conn, mut rx)) = connect.await {
            let started = this.update(cx, |s, cx| {
                cx.set_global(ShellConnection(conn.clone()));
                // at once, not at the next render: a shell in front without a theme blanks herdr's, and a window
                // that is not drawn (screen locked, another Space) has no render to send it from
                let theme = Theme::of(cx);
                conn.theme(host_theme(theme));
                let theme = Some(theme.terminal.background);
                s.client_shell = Some(ShellView { conn, size: Some((cols, rows)), theme, active: true });
                cx.notify();
            });
            if started.is_err() {
                return;
            }
            while let Some(event) = rx.next().await {
                let alive = this.update(cx, |s, cx| match event {
                    PopupEvent::Popup(surface) => s.show_popup(surface, cx),
                    PopupEvent::Commands(commands) => s.herdr_commands = commands,
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
    /// Tells herdr what changed since the last render: the grid's size and the theme. Coming back to the window
    /// makes the shell herdr's client in front again, so herdr takes Shepherd's theme back from its TUI.
    /// Not sent on purpose: `tab.focus` on a tab switch (it moves herdr's own focus too; only a command does, see
    /// `focus_shown_tab`) and `ClientShellFocus` (the client in front's focus marks herdr's active tab seen and mutes
    /// its notifications).
    pub(super) fn sync_client_shell(&mut self, window: &Window, cx: &App) {
        let cells = self.grid_cells(cx);
        let Some(view) = &mut self.client_shell else { return };
        let active = window.is_window_active();
        let came_back = active && !view.active;
        view.active = active;
        if let Some((cols, rows)) = cells.filter(|c| came_back || view.size != Some(*c)) {
            // a resize, even to the same size, puts the shell in front
            view.conn.resize(cols, rows);
            view.size = Some((cols, rows));
        }
        let theme = Theme::of(cx);
        if view.theme != Some(theme.terminal.background) {
            view.conn.theme(host_theme(theme));
            view.theme = Some(theme.terminal.background);
        }
    }
}

impl Shell {
    /// herdr gives a popup to the tab in its focus as the popup opens, and only a client shell on that tab gets it.
    /// Shepherd shows tabs on its own, so before a command runs, the shell's tab (and herdr's focus) moves to the shown
    /// one: a popup the command opens comes to this window, and Shepherd's `pane.focus` (an agent seen) keeps it there.
    pub(super) fn focus_shown_tab(&self, cx: &App) -> Option<gpui::Task<()>> {
        let (conn, tab) = (self.client_shell.as_ref()?.conn.clone(), self.shown_tab()?);
        Some(cx.background_executor().spawn(async move { conn.focus_tab(&tab.tab_id) }))
    }
}

pub(super) fn host_theme(theme: &Theme) -> Vec<Update> {
    use crate::herdr::client_shell::{DefaultColorKind as Kind, HostAppearance as Appearance, Rgb};
    let rgb = |c: Hsla| {
        let c = gpui::Rgba::from(c);
        let b = |v: f32| (v.clamp(0.0, 1.0) * 255.0).round() as u8;
        Rgb { r: b(c.r), g: b(c.g), b: b(c.b) }
    };
    vec![
        Update::DefaultColor { kind: Kind::Foreground, color: rgb(theme.terminal.foreground) },
        Update::DefaultColor { kind: Kind::Background, color: rgb(theme.terminal.background) },
        Update::PaletteColors(theme.terminal.ansi.iter().enumerate().map(|(i, c)| (i as u8, rgb(*c))).collect()),
        Update::Appearance(if theme.appearance == crate::theme::Appearance::Dark { Appearance::Dark } else { Appearance::Light }),
    ]
}

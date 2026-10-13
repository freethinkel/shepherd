//! herdr's popup (a plugin's chooser, a `[[keys.command]]` popup) as a floating terminal over the panes.
use super::*;
use crate::herdr::popup::{self, Event, PopupSurface};
use futures::StreamExt;

/// The popup shown: what herdr says of it, and its terminal, controlled like a pane's.
pub(super) struct Floating {
    surface: PopupSurface,
    /// Only while this window is the active one: herdr lets one client type into the popup, so of Shepherd's
    /// windows the one in front takes it.
    view: Option<Entity<PaneTerminal>>,
}

/// herdr's popup as a client-rendered shell hears of it; reconnects while the window follows this session.
pub(super) async fn popup_loop(this: gpui::WeakEntity<Shell>, cx: &mut gpui::AsyncApp) {
    loop {
        // the shell's surface sizes herdr's focused tab while no other client does: make it the pane grid
        let Ok((cols, rows)) = this.update(cx, |s, cx| s.grid_cells(cx).unwrap_or((120, 40))) else { return };
        let watch = cx.background_executor().spawn(async move { popup::watch(cols, rows) });
        if let Ok((mut rx, surface)) = watch.await {
            if this.update(cx, |s, _| s.popup_surface = Some((surface, (cols, rows)))).is_err() {
                return;
            }
            while let Some(event) = rx.next().await {
                let alive = this.update(cx, |s, cx| match event {
                    Event::Popup(surface) => s.show_popup(surface, cx),
                    Event::Commands(commands) => s.herdr_commands = commands,
                });
                if alive.is_err() {
                    return;
                }
            }
        }
        let gone = this.update(cx, |s, cx| {
            s.popup_surface = None;
            s.show_popup(None, cx)
        });
        if gone.is_err() {
            return;
        }
        // ponytail: retry every 5 s (herdr restarting, a remote place whose client socket is not forwarded)
        cx.background_executor().timer(Duration::from_secs(5)).await;
    }
}

impl Shell {
    /// The pane grid in cells, once a pane has measured its font.
    fn grid_cells(&self, cx: &App) -> Option<(u16, u16)> {
        let bounds = self.grid_bounds.get()?;
        let g = self.panes.values().find_map(|p| p.read(cx).geometry())?;
        Some(((f32::from(bounds.size.width) / g.cell_w) as u16, (f32::from(bounds.size.height) / g.line_h) as u16))
    }

    fn show_popup(&mut self, surface: Option<PopupSurface>, cx: &mut Context<Self>) {
        match (surface, &mut self.floating) {
            (None, _) => {
                if self.floating.take().is_some() {
                    self.focused = None; // the keyboard goes back to herdr's focused pane
                }
            }
            (Some(surface), Some(f)) if f.surface.terminal_id == surface.terminal_id => f.surface = surface,
            (Some(surface), _) => self.floating = Some(Floating { surface, view: None }),
        }
        cx.notify();
    }

    pub(super) fn floating_open(&self) -> bool {
        self.floating.is_some()
    }

    /// The popup's card, centred over the pane grid; a press outside it closes the popup, as Esc in a dialog.
    pub(super) fn render_floating(&mut self, theme: &Theme, window: &mut Window, cx: &mut Context<Self>) -> Option<impl IntoElement + use<>> {
        let grid = self.grid_cells(cx);
        if let (Some((surface, sent)), Some(now)) = (&mut self.popup_surface, grid) {
            if *sent != now {
                surface.resize(now.0, now.1);
                *sent = now;
            }
        }
        let f = self.floating.as_mut()?;
        if !window.is_window_active() {
            f.view = None;
            return None;
        }
        let view = match &f.view {
            Some(view) => view.clone(),
            None => {
                let id = f.surface.terminal_id.clone();
                let view = cx.new(|cx| {
                    let mut pane = PaneTerminal::new(id, cx);
                    pane.takes_over = true;
                    pane
                });
                window.focus(&view.read(cx).focus_handle(cx), cx);
                f.view = Some(view.clone());
                view
            }
        };
        let (title, width, height) = (f.surface.title.clone(), f.surface.width, f.surface.height);
        let g = self.panes.values().find_map(|p| p.read(cx).geometry());
        let (cols, rows) = self.grid_cells(cx).unwrap_or((120, 40));
        let (w, h) = popup::size(width, height, cols, rows)?;
        let (cell_w, line_h) = g.map_or((8., 17.), |g| (g.cell_w, g.line_h));
        const TITLE: f32 = 26.;
        Some(
            div().absolute().inset_0().flex().items_center().justify_center().child(
                div()
                    .id("herdr-popup")
                    .occlude()
                    .w(px(w as f32 * cell_w))
                    .h(px(h as f32 * line_h + TITLE))
                    .flex()
                    .flex_col()
                    .rounded(px(10.))
                    .overflow_hidden()
                    .bg(theme.terminal.background)
                    .border_1()
                    .border_color(theme.border_strong)
                    .shadow_lg()
                    .on_mouse_down_out(cx.listener(|this, _: &MouseDownEvent, _, cx| {
                        let client = this.client.clone();
                        this.rt.spawn(async move { client.call("popup.close", json!({})).await });
                        cx.notify();
                    }))
                    .child(
                        div()
                            .h(px(TITLE))
                            .flex_none()
                            .flex()
                            .items_center()
                            .justify_center()
                            .text_size(px(12.))
                            .text_color(theme.text_muted)
                            .border_b_1()
                            .border_color(theme.border)
                            .child(title),
                    )
                    .child(div().flex_1().min_h_0().child(view)),
            ),
        )
    }
}

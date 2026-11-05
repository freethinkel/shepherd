//! A blank browser tab's body: what this Mac's dev servers listen on, the shown workspace's first. A click loads the
//! port in the tab; Stop sends the process SIGTERM.
use super::*;
use super::web::PageKey;
use crate::ports::{Listener, read, under};
use crate::ui::statusbar::{cached, fetching, refetch, short_path};
use gpui::AnyElement;

const KEY: &str = "ports";
/// How often `lsof` is asked while the list is shown.
const EVERY: Duration = Duration::from_secs(2);

impl Shell {
    /// A blank browser tab in the opened panel: `SHEPHERD_PANEL=browser` for a screenshot.
    pub fn open_blank_tab(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(tab) = self.shown_tab() {
            self.new_browser_tab(tab.workspace_id, None, false, window, cx);
        }
    }

    /// Ports are read here, with this Mac's `lsof`: a session on another machine has its own.
    fn ports_offered(&self) -> bool {
        cfg!(unix) && self.machine_label.is_none()
    }

    /// Whether browser tab `key` shows the ports in place of its page: nothing loaded in it yet.
    pub(super) fn shows_ports(&self, key: &PageKey) -> bool {
        let url = self.pages.live.get(key).and_then(|p| p.view.url().ok());
        self.ports_offered() && browser::is_tab(&key.0) && url.is_none_or(|u| u.is_empty() || u == browser::BLANK)
    }

    /// The directories the shown workspace's panes are in.
    fn workspace_dirs(&self) -> Vec<String> {
        let (Some(snapshot), Some(tab)) = (self.snapshot.as_ref(), self.shown_tab()) else { return Vec::new() };
        let panes = snapshot.panes.iter().filter(|p| p.workspace_id == tab.workspace_id);
        panes.filter_map(|p| p.foreground_cwd.clone().or_else(|| p.cwd.clone())).filter(|d| !d.is_empty()).collect()
    }

    pub(super) fn render_ports(&mut self, key: &PageKey, theme: &Theme, window: &mut Window, cx: &mut Context<Self>) -> AnyElement {
        let listeners = cached(KEY, EVERY, read, cx);
        // asked again while shown: one timer at a time, and none once the list is gone
        if !self.browser.ports_tick {
            self.browser.ports_tick = true;
            cx.spawn(async move |this, cx| {
                cx.background_executor().timer(EVERY).await;
                let _ = this.update(cx, |shell, cx| {
                    shell.browser.ports_tick = false;
                    cx.notify();
                });
            })
            .detach();
        }
        let body: Vec<AnyElement> = match listeners {
            None if fetching::<Vec<Listener>>(KEY, cx) => vec![div().pt(px(24.)).flex().justify_center().child(crate::ui::loader::spinner(2.5, window)).into_any_element()],
            None => vec![note(theme, "lsof didn't run.")],
            Some(l) if l.is_empty() => vec![note(theme, "Nothing started from a project folder listens on a TCP port.")],
            Some(listeners) => {
                let dirs = self.workspace_dirs();
                let (here, other): (Vec<_>, Vec<_>) = listeners.into_iter().partition(|l| dirs.iter().any(|d| under(&l.cwd, d)));
                let home = std::env::home_dir();
                let mut rows = Vec::new();
                for (label, group) in [("This workspace", here), ("Elsewhere", other)] {
                    if !group.is_empty() {
                        rows.push(heading(theme, label));
                        rows.extend(group.into_iter().map(|l| port_row(&l, key, home.as_deref(), theme, cx)));
                    }
                }
                rows
            }
        };
        div()
            .id("ports-list")
            .flex_1()
            .min_h_0()
            .bg(theme.terminal.background)
            .overflow_y_scroll()
            .p(px(8.))
            .child(div().px(px(8.)).pt(px(8.)).pb(px(4.)).font_weight(gpui::FontWeight::MEDIUM).text_color(theme.text).child("Open ports"))
            .children(body)
            .into_any_element()
    }
}

fn port_row(l: &Listener, key: &PageKey, home: Option<&std::path::Path>, theme: &Theme, cx: &mut Context<Shell>) -> AnyElement {
    let (url, pid, key) = (l.url(), l.pid, key.clone());
    let group = SharedString::from(format!("port-{}-{pid}", l.port));
    let hover = theme.hover;
    let stop = div()
        .id(SharedString::from(format!("port-stop-{}-{pid}", l.port)))
        .size(px(24.))
        .flex_none()
        .rounded(px(6.))
        .flex()
        .items_center()
        .justify_center()
        .invisible()
        .group_hover(group.clone(), |s| s.visible())
        .hover(move |s| s.bg(hover))
        .tooltip(tip("Stop", None))
        .child(gpui::svg().path("icons/solar/stop-circle.svg").size(px(14.)).text_color(theme.text_muted))
        .on_click(move |_: &ClickEvent, _, cx| {
            cx.stop_propagation();
            let _ = crate::process::command("kill").arg(pid.to_string()).status();
            refetch::<Vec<Listener>>(KEY, cx);
        });
    div()
        .id(group.clone())
        .group(group)
        .h(px(44.))
        .px(px(8.))
        .rounded(px(8.))
        .flex()
        .items_center()
        .gap(px(10.))
        .cursor_pointer()
        .hover(move |s| s.bg(hover))
        .child(div().w(px(56.)).flex_none().font_family(theme.font_terminal.clone()).text_color(theme.text).child(format!(":{}", l.port)))
        .child(
            div()
                .flex_1()
                .min_w_0()
                .flex()
                .flex_col()
                .child(div().truncate().text_size(px(13.)).text_color(theme.text).child(l.name.clone()))
                .child(div().truncate().text_size(px(11.)).text_color(theme.text_muted).child(short_path(&l.cwd, home))),
        )
        .child(stop)
        .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
            if let Some(page) = this.pages.live.get(&key) {
                let _ = page.view.load_url(&url);
            }
            cx.notify();
        }))
        .into_any_element()
}

fn heading(theme: &Theme, label: &'static str) -> AnyElement {
    div().px(px(8.)).pt(px(12.)).pb(px(4.)).text_size(px(11.)).text_color(theme.text_faint).child(label).into_any_element()
}

fn note(theme: &Theme, text: &'static str) -> AnyElement {
    div().px(px(8.)).pt(px(8.)).text_size(px(12.)).text_color(theme.text_muted).child(text).into_any_element()
}

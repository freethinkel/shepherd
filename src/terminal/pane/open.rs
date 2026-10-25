//! Cmd+click: links to the browser or the editor, images to the viewer over the pane, Markdown to the reader.
use super::*;

pub struct Preview {
    path: PathBuf,
    /// In pixels, from the file's header; none for an SVG or a file that does not decode.
    size: Option<(u32, u32)>,
    /// Times the fit.
    zoom: f32,
    /// The image's middle from the frame's, in px.
    pan: (f32, f32),
    /// Where the pointer was, while a drag moves the image.
    drag: Option<gpui::Point<Pixels>>,
    opened: std::time::Instant,
}

/// Quick Look's entrance and exit for a dialog opened at `opened`: the card grows into place as it fades in, and
/// shrinks a little leaving. 1 is in place.
pub(super) fn grown<T>(popup: &Popup<T>, opened: std::time::Instant, window: &Window) -> f32 {
    let opening = (opened.elapsed().as_secs_f32() / PREVIEW_IN.as_secs_f32()).min(1.);
    let closing = popup.closed_for().map_or(0., |d| (d.as_secs_f32() / 0.1).min(1.));
    if opening < 1. {
        window.request_animation_frame();
    }
    0.9 + 0.1 * (1. - (1. - opening).powi(3)) - 0.04 * closing
}

/// A Markdown file in the reader.
pub struct Reader {
    file: File,
    /// The pane's shell's folder, where Neovim opens beside it.
    cwd: String,
    /// The file's Markdown, for the copy button.
    text: String,
    blocks: Vec<crate::ui::review::Block>,
    /// The images fetched so far, by their source in the file.
    images: std::sync::Arc<std::collections::HashMap<String, PathBuf>>,
    scroll: gpui::ScrollHandle,
    opened: std::time::Instant,
    /// The copy button shows a check for a moment after a copy.
    copied: bool,
}

/// The images' sources in `blocks`, each once, in document order.
pub(super) fn image_sources(blocks: &[crate::ui::review::Block], out: &mut Vec<String>) {
    use crate::ui::review::Block;
    let runs = |runs: &[crate::ui::review::InlineRun], out: &mut Vec<String>| {
        for image in runs.iter().filter_map(|r| r.style.image.as_ref()) {
            if !out.contains(&image.source) {
                out.push(image.source.clone());
            }
        }
    };
    for block in blocks {
        match block {
            Block::Paragraph { runs: r } | Block::Heading { runs: r, .. } => runs(r, out),
            Block::BlockQuote { children } => image_sources(children, out),
            Block::List { items, .. } => items.iter().for_each(|item| image_sources(item, out)),
            Block::Table { header, rows, .. } => header.iter().chain(rows.iter().flatten()).for_each(|cell| runs(cell, out)),
            _ => {}
        }
    }
}

/// A web image, downloaded into the cache under the extension its type has (a badge's URL often has none).
pub(super) async fn download(url: &str) -> Option<PathBuf> {
    let url = url.to_string();
    let fetched = tokio::task::spawn_blocking(move || {
        let agent: ureq::Agent = ureq::Agent::config_builder().timeout_global(Some(std::time::Duration::from_secs(15))).build().into();
        let mut response = agent.get(&url).call().ok()?;
        let kind = response.headers().get("content-type").and_then(|v| v.to_str().ok()).unwrap_or_default().to_string();
        let bytes = response.body_mut().with_config().limit(32 << 20).read_to_vec().ok()?;
        let ext = ["svg", "png", "gif", "webp", "jpeg"].into_iter().find(|e| kind.contains(e)).unwrap_or("png");
        Some((bytes, format!("image.{ext}")))
    });
    let (bytes, name) = fetched.await.ok()??;
    cache_copy(&name, bytes).await
}

/// `bytes` as a file named `name` in the cache, under their hash: gpui keeps an image by its path, so a changed file
/// needs a new one.
// ponytail: copies pile up in the cache until it is cleared; prune by age if they ever matter.
pub(super) async fn cache_copy(name: &str, bytes: Vec<u8>) -> Option<PathBuf> {
    let hash = {
        use std::hash::{Hash, Hasher};
        let mut h = std::collections::hash_map::DefaultHasher::new();
        bytes.hash(&mut h);
        h.finish()
    };
    let dir = crate::settings::cache_dir().join("files").join(format!("{hash:016x}"));
    tokio::fs::create_dir_all(&dir).await.ok()?;
    let copy = dir.join(name);
    tokio::fs::write(&copy, bytes).await.ok()?;
    Some(copy)
}

/// The reader's markdown selection keys start with it.
pub(super) const READER_KEY: &str = "reader-md";

/// What a Cmd+click opens over the pane.
pub(super) enum Opened {
    Image(PathBuf),
    Markdown { file: File, cwd: String, text: String, blocks: Vec<crate::ui::review::Block> },
}

/// A clicked file where the pane's shell runs: here, or on the machine at `machine` (its SSH target).
#[derive(Clone)]
pub(super) struct File {
    path: PathBuf,
    dir: bool,
    machine: Option<String>,
}

impl File {
    pub(super) async fn find(machine: Option<String>, path: &str, cwd: &str) -> Option<File> {
        let Some(target) = machine else {
            let path = resolve(path, Path::new(cwd))?;
            return Some(File { dir: path.is_dir(), path, machine: None });
        };
        let (t, path, cwd) = (target.clone(), path.to_string(), cwd.to_string());
        let (found, dir) = tokio::task::spawn_blocking(move || crate::herdr::find_file(&t, &cwd, &path)).await.ok()??;
        Some(File { path: PathBuf::from(found), dir, machine: Some(target) })
    }

    /// Its bytes, a machine's over SSH.
    pub(super) async fn bytes(&self) -> Option<Vec<u8>> {
        let Some(target) = self.machine.clone() else { return tokio::fs::read(&self.path).await.ok() };
        let path = self.path.to_string_lossy().into_owned();
        tokio::task::spawn_blocking(move || crate::herdr::read_file(&target, &path)).await.ok()?.ok()
    }

    /// The file as one here for the image viewer: itself, or a machine's copied into the cache.
    pub(super) async fn local_copy(&self) -> Option<PathBuf> {
        if self.machine.is_none() {
            return Some(self.path.clone());
        }
        let name = self.path.file_name()?.to_string_lossy().into_owned();
        cache_copy(&name, self.bytes().await?).await
    }
}

/// Opens `file` at its line in the user's editor: Neovim in a pane to the right of `pane_id`, where the pane's shell
/// runs; another editor here, over its SSH remoting for a machine's file.
pub(super) async fn edit(client: &Client, pane_id: &str, cwd: &str, editor: Option<&str>, file: &File, line: Option<u32>, col: Option<u32>) {
    if is_nvim(editor) {
        // A terminal editor gets a pane to the right; its shell drops typeahead until the prompt is drawn.
        let params = serde_json::json!({"target_pane_id": pane_id, "direction": "right", "focus": true, "cwd": cwd});
        let Ok(r) = client.call("pane.split", params).await else { return };
        let Some(new_pane) = r["pane"]["pane_id"].as_str() else { return };
        let prompt = serde_json::json!({"type": "regex", "value": "\\S"});
        let wait = serde_json::json!({"pane_id": new_pane, "source": "recent", "match": prompt, "timeout_ms": 5000});
        let _ = client.call("pane.wait_for_output", wait).await;
        let text = nvim_command(editor.unwrap_or("nvim"), &file.path, line, col);
        let _ = client.call("pane.send_input", serde_json::json!({"pane_id": new_pane, "text": text, "keys": ["enter"]})).await;
        return;
    }
    let Some(argv) = editor_argv(editor, file.machine.as_deref(), &file.path, line, col) else { return };
    if let Ok(mut child) = tokio::process::Command::new(&argv[0]).args(&argv[1..]).spawn() {
        let _ = child.wait().await;
    }
}

pub(super) const MIN_ZOOM: f32 = 0.1;
pub(super) const MAX_ZOOM: f32 = 20.;
pub(super) const ZOOM_STEP: f32 = 1.25;
pub(super) const PREVIEW_IN: std::time::Duration = std::time::Duration::from_millis(220);
pub(super) const PREVIEW_PAD: f32 = 8.;
pub(super) const PREVIEW_HEADER: f32 = 22.;
pub(super) const PREVIEW_GAP: f32 = 6.;

/// The image dialog's and the reader's card, `frame`'s size.
pub(super) fn dialog_card(id: &'static str, frame: &PreviewFrame, theme: &Theme) -> gpui::Stateful<gpui::Div> {
    div()
        .id(id)
        .w(px(frame.card.0))
        .h(px(frame.card.1))
        .p(px(PREVIEW_PAD))
        .rounded(px(12.))
        .bg(theme.card)
        .border_1()
        .border_color(theme.border)
        .shadow_lg()
        .flex()
        .flex_col()
        .gap(px(PREVIEW_GAP))
        .text_color(theme.text)
}

/// A dialog's header row: the file's name, then the controls added to it.
pub(super) fn dialog_header(name: String, theme: &Theme) -> gpui::Div {
    div()
        .h(px(PREVIEW_HEADER))
        .flex_none()
        .flex()
        .items_center()
        .gap(px(2.))
        .pl(px(6.))
        .child(div().flex_1().min_w_0().truncate().text_size(px(12.)).text_color(theme.text_muted).child(name))
}

/// A header's icon button, ghost until hovered.
pub(super) fn dialog_button(id: &'static str, icon: &'static str, theme: &Theme) -> gpui::Stateful<gpui::Div> {
    div()
        .id(id)
        .flex_none()
        .size(px(22.))
        .rounded(px(6.))
        .flex()
        .items_center()
        .justify_center()
        .cursor_pointer()
        .hover(|s| s.bg(theme.hover))
        .child(gpui::svg().path(icon).size(px(14.)).text_color(theme.text_muted))
}

/// The dialog's geometry in a window: the card's size, its image frame's, and the frame's middle on the window.
pub(super) struct PreviewFrame {
    card: (f32, f32),
    area: (f32, f32),
    center: (f32, f32),
}

impl PreviewFrame {
    /// The geometry `grown` (1: in place) of the way in.
    pub(super) fn of(window: &Window, grown: f32) -> Self {
        let v = window.viewport_size();
        let (vw, vh) = (f32::from(v.width), f32::from(v.height));
        let card = ((vw * 0.85).max(320.) * grown, (vh * 0.85).max(240.) * grown);
        // the card's border is 1 px a side
        let area = (card.0 - 2. * PREVIEW_PAD - 2., card.1 - 2. * PREVIEW_PAD - 2. - PREVIEW_HEADER - PREVIEW_GAP);
        let center = (vw / 2., (vh - card.1) / 2. + 1. + PREVIEW_PAD + PREVIEW_HEADER + PREVIEW_GAP + area.1 / 2.);
        PreviewFrame { card, area, center }
    }

    /// A window point from the frame's middle.
    pub(super) fn at(&self, p: gpui::Point<Pixels>) -> (f32, f32) {
        (f32::from(p.x) - self.center.0, f32::from(p.y) - self.center.1)
    }
}

impl Preview {
    /// The fit: inside the frame, never past the image's own pixels. An SVG fills the frame.
    pub(super) fn fit(&self, f: &PreviewFrame) -> (f32, f32) {
        match self.size {
            Some((w, h)) => {
                let k = (f.area.0 / w as f32).min(f.area.1 / h as f32).min(1.);
                (w as f32 * k, h as f32 * k)
            }
            None => f.area,
        }
    }

    pub(super) fn shown(&self, f: &PreviewFrame) -> (f32, f32) {
        let (w, h) = self.fit(f);
        (w * self.zoom, h * self.zoom)
    }

    /// Shown px per image pixel (per the fit, for an SVG).
    pub(super) fn scale(&self, f: &PreviewFrame) -> f32 {
        self.size.map_or(self.zoom, |(w, _)| self.shown(f).0 / w as f32)
    }

    /// Scales to `zoom` keeping the image's point under `at` (from the frame's middle) in place, as far as
    /// the frame lets it.
    pub(super) fn zoom_about(&mut self, zoom: f32, at: (f32, f32), f: &PreviewFrame) {
        let zoom = zoom.clamp(MIN_ZOOM, MAX_ZOOM);
        let k = zoom / self.zoom;
        let pan = (at.0 - (at.0 - self.pan.0) * k, at.1 - (at.1 - self.pan.1) * k);
        (self.zoom, self.pan) = (zoom, (0., 0.));
        self.pan_by(pan, f);
    }

    /// Moves the image by `d`, Preview's way: a larger one keeps the frame covered, a smaller one stays inside it.
    pub(super) fn pan_by(&mut self, d: (f32, f32), f: &PreviewFrame) {
        let (w, h) = self.shown(f);
        let (mx, my) = ((w - f.area.0).abs() / 2., (h - f.area.1).abs() / 2.);
        self.pan = ((self.pan.0 + d.0).clamp(-mx, mx), (self.pan.1 + d.1).clamp(-my, my));
    }
}

/// Whether the viewer can show the file: the formats gpui decodes.
pub(super) fn is_image(path: &Path) -> bool {
    let ext = path.extension().and_then(|e| e.to_str()).map(str::to_ascii_lowercase);
    matches!(ext.as_deref(), Some("png" | "jpg" | "jpeg" | "gif" | "webp" | "bmp" | "tif" | "tiff" | "svg" | "ico"))
}

/// Whether the reader shows the file.
pub(super) fn is_markdown(path: &Path) -> bool {
    let ext = path.extension().and_then(|e| e.to_str()).map(str::to_ascii_lowercase);
    matches!(ext.as_deref(), Some("md" | "markdown" | "mdx"))
}

/// A clicked path as a file that exists: `~` is home, a relative path is the pane's shell's.
pub(super) fn resolve(path: &str, cwd: &Path) -> Option<PathBuf> {
    let path = match path.strip_prefix("~/") {
        Some(rest) => std::env::home_dir()?.join(rest),
        None => cwd.join(path),
    };
    path.exists().then_some(path)
}

/// The command opening `path` at its line: the user's editor, else the first of Zed, Cursor and VS Code installed,
/// else macOS's default app for the file (which cannot take a line). A file on the machine at `remote` (its SSH
/// target) opens over the editor's SSH remoting, Zed's `ssh://` URL or VS Code's and Cursor's `--remote`; `None`
/// for an editor without it.
pub fn editor_argv(editor: Option<&str>, remote: Option<&str>, path: &Path, line: Option<u32>, col: Option<u32>) -> Option<Vec<String>> {
    let target = match (line, col) {
        (Some(l), Some(c)) => format!("{}:{l}:{c}", path.display()),
        (Some(l), None) => format!("{}:{l}", path.display()),
        _ => path.display().to_string(),
    };
    // An app opened from Finder gets launchd's PATH, without the usual homes of these CLIs.
    let path_env = std::env::var("PATH").unwrap_or_default();
    let installed = |name: &str| {
        let dirs = path_env.split(':').chain(["/usr/local/bin", "/opt/homebrew/bin"]);
        dirs.map(|d| Path::new(d).join(name)).find(|p| p.is_file()).map(|p| p.display().to_string())
    };
    let words = match editor.map(|e| e.split_whitespace().map(String::from).collect::<Vec<_>>()).filter(|w| !w.is_empty()) {
        Some(words) => {
            let program = if words[0].contains('/') { None } else { installed(&words[0]) };
            [vec![program.unwrap_or_else(|| words[0].clone())], words[1..].to_vec()].concat()
        }
        None => match (installed("zed"), installed("cursor").or_else(|| installed("code"))) {
            (Some(zed), _) => vec![zed],
            (None, Some(code)) => vec![code, "-g".into()],
            (None, None) => return remote.is_none().then(|| vec!["open".into(), path.display().to_string()]),
        },
    };
    let Some(host) = remote.map(|r| r.trim_start_matches("ssh://")) else { return Some([words, vec![target]].concat()) };
    match Path::new(&words[0]).file_name().and_then(|n| n.to_str()) {
        Some("zed") => Some([words, vec![format!("ssh://{host}{target}")]].concat()),
        Some("code" | "cursor" | "codium" | "windsurf") => {
            // `-g` wants a line; a folder or a whole file opens without it
            let flags = words[1..].iter().filter(|w| line.is_some() || *w != "-g").cloned();
            let remote = ["--remote".to_string(), format!("ssh-remote+{host}")];
            Some(words[..1].iter().cloned().chain(remote).chain(flags).chain([target]).collect())
        }
        _ => None,
    }
}

/// Whether the editor command runs nvim, which opens in a pane instead of its own window.
pub fn is_nvim(editor: Option<&str>) -> bool {
    editor.and_then(|e| e.split_whitespace().next()).is_some_and(|w| w.rsplit('/').next() == Some("nvim"))
}

/// The shell line opening `path` at its line and column in nvim; `exec`, so quitting nvim closes its pane.
pub fn nvim_command(editor: &str, path: &Path, line: Option<u32>, col: Option<u32>) -> String {
    let quoted = format!("'{}'", path.display().to_string().replace('\'', "'\\''"));
    let jump = match (line, col) {
        (Some(l), Some(c)) => format!(" '+call cursor({l},{c})'"),
        (Some(l), None) => format!(" +{l}"),
        _ => String::new(),
    };
    format!("exec {editor}{jump} {quoted}")
}

impl PaneTerminal {
    /// The link under a window position: its row, cells and target.
    pub(super) fn link_under(&self, position: gpui::Point<Pixels>) -> Option<(usize, Range<usize>, Link)> {
        let g = self.geometry?;
        if !g.bounds.contains(&position) {
            return None;
        }
        let (row, col, _, _) = self.hit(position)?;
        // An OSC 8 hyperlink wins over the text under it.
        let osc8 = self.emulator.hyperlink_at(row as usize, col as usize);
        if let Some((cells, link)) = osc8.and_then(|(cells, uri)| Some((cells, hyperlink(&uri)?))) {
            return Some((row as usize, cells, link));
        }
        let cells = self.emulator.line(row as usize);
        let (chars, cols) = row_chars(&cells);
        let at = cols.iter().rposition(|&c| c <= col as usize)?;
        let (chars_range, link) = link_at(&chars, at)?;
        Some((row as usize, cell_range(&cells, &cols, chars_range), link))
    }

    pub(super) fn update_hover(&mut self, cmd: bool, cx: &mut Context<Self>) {
        // No links under the image dialog: Cmd, still held from the click that opened it, would point at them.
        let hover = self.pointer.filter(|_| cmd && self.preview.is_none() && self.reader.is_none()).and_then(|p| self.link_under(p)).map(|(row, cells, _)| (row, cells));
        if hover != self.hover {
            self.hover = hover;
            cx.notify();
        }
    }

    /// URLs go to the browser (`file://` ones to their default app); paths that exist where the pane's shell runs
    /// (here, or on the machine shown) to the editor; images, either way, to the viewer over the pane, and Markdown
    /// files to the reader (its header opens them in the editor).
    pub(super) fn open_link(&mut self, link: Link, cx: &mut Context<Self>) {
        let (path, line, col) = match link {
            // `ls --hyperlink` percent-encodes the path (a space is `%20`).
            Link::Url(url) => match url
                .strip_prefix("file://")
                .map(|p| percent_encoding::percent_decode_str(p).decode_utf8_lossy().into_owned())
                .filter(|p| is_image(Path::new(p)) || is_markdown(Path::new(p)))
            {
                Some(path) => (path, None, None),
                None => return crate::shell::open_link(&url, cx),
            },
            Link::File { path, line, col } => (path, line, col),
        };
        let Some(PaneQueries(client, rt)) = cx.try_global::<PaneQueries>() else { return };
        let editor = cx.try_global::<crate::settings::SettingsGlobal>().and_then(|s| s.settings.editor.clone());
        let machine = crate::herdr::shown_machine();
        let (client, pane_id) = (client.clone(), self.pane_id.clone());
        let opened = rt.spawn(async move {
            let pane = client.call("pane.get", serde_json::json!({"pane_id": pane_id})).await.map(|r| r["pane"].clone()).unwrap_or_default();
            let cwd = pane["foreground_cwd"].as_str().or(pane["cwd"].as_str()).unwrap_or("/").to_string();
            let file = File::find(machine, &path, &cwd).await?;
            if is_image(&file.path) {
                return file.local_copy().await.map(Opened::Image);
            }
            if !file.dir && is_markdown(&file.path) {
                let text = String::from_utf8_lossy(&file.bytes().await?).into_owned();
                let blocks = crate::ui::review::markdown(&text);
                return Some(Opened::Markdown { file, cwd, text, blocks });
            }
            // a folder here goes to Finder, not the editor
            if file.dir && file.machine.is_none() {
                let _ = tokio::process::Command::new("open").arg(&file.path).status().await;
                return None;
            }
            edit(&client, &pane_id, &cwd, editor.as_deref(), &file, line, col).await;
            None
        });
        cx.spawn(async move |this, cx| match opened.await {
            Ok(Some(Opened::Image(path))) => {
                let _ = this.update(cx, |pane, cx| pane.show_preview(path, cx));
            }
            Ok(Some(Opened::Markdown { file, cwd, text, blocks })) => {
                let _ = this.update(cx, |pane, cx| pane.show_reader(file, cwd, text, blocks, cx));
            }
            _ => {}
        })
        .detach();
    }

    pub(super) fn show_reader(&mut self, file: File, cwd: String, text: String, blocks: Vec<crate::ui::review::Block>, cx: &mut Context<Self>) {
        let opened = std::time::Instant::now();
        let reader = Reader { file, cwd, text, blocks, images: Default::default(), scroll: gpui::ScrollHandle::new(), opened, copied: false };
        self.reader = Some(Popup::new(reader));
        self.hover = None;
        self.load_images(cx);
        cx.notify();
    }

    /// Fetches the reader's images into files here, a machine's over SSH and a web one over HTTPS, and shows each
    /// set as it arrives; until then their alt text stands in.
    pub(super) fn load_images(&mut self, cx: &mut Context<Self>) {
        let Some(r) = self.reader.as_ref().map(|r| &r.value) else { return };
        let mut sources = Vec::new();
        image_sources(&r.blocks, &mut sources);
        let Some(PaneQueries(_, rt)) = cx.try_global::<PaneQueries>().filter(|_| !sources.is_empty()) else { return };
        let (dir, machine, opened) = (r.file.path.parent().map(Path::to_path_buf).unwrap_or_default(), r.file.machine.clone(), r.opened);
        // ponytail: one image after another; in parallel if a page with many over SSH ever loads slowly
        let loaded = rt.spawn(async move {
            let mut images = std::collections::HashMap::new();
            for source in sources {
                let path = if source.starts_with("https://") || source.starts_with("http://") {
                    download(&source).await
                } else {
                    let path = dir.join(percent_encoding::percent_decode_str(&source).decode_utf8_lossy().as_ref());
                    File { path, dir: false, machine: machine.clone() }.local_copy().await
                };
                if let Some(path) = path {
                    images.insert(source, path);
                }
            }
            images
        });
        cx.spawn(async move |this, cx| {
            let Ok(images) = loaded.await else { return };
            let _ = this.update(cx, |pane, cx| {
                if let Some(reader) = pane.reader.as_mut().filter(|r| r.value.opened == opened) {
                    reader.value.images = std::sync::Arc::new(images);
                    cx.notify();
                }
            });
        })
        .detach();
    }

    /// A link clicked in the reader: a web one to the browser; a path, from the file's folder, opened as a Cmd+click
    /// opens it (another Markdown file in the reader, `#L12` at its line); a bare `#anchor` goes nowhere.
    pub(super) fn follow_link(&mut self, url: &str, cx: &mut Context<Self>) {
        if url.contains("://") || url.starts_with("mailto:") {
            return cx.open_url(url);
        }
        let (path, anchor) = url.split_once('#').unwrap_or((url, ""));
        let Some(dir) = self.reader.as_ref().and_then(|r| r.value.file.path.parent().map(Path::to_path_buf)).filter(|_| !path.is_empty()) else {
            return;
        };
        let path = dir.join(percent_encoding::percent_decode_str(path).decode_utf8_lossy().as_ref());
        let line = anchor.strip_prefix('L').and_then(|l| l.split('-').next()?.parse().ok());
        self.open_link(Link::File { path: path.to_string_lossy().into_owned(), line, col: None }, cx);
    }

    pub(super) fn close_reader(&mut self, cx: &mut Context<Self>) {
        if let Some(reader) = &mut self.reader {
            reader.close();
            crate::ui::markdown::forget(READER_KEY);
            cx.notify();
        }
    }

    /// The reader's Markdown to the clipboard, the button checked for a moment.
    pub(super) fn copy_reader(&mut self, cx: &mut Context<Self>) {
        let Some(reader) = self.reader.as_mut() else { return };
        cx.write_to_clipboard(gpui::ClipboardItem::new_string(reader.value.text.clone()));
        reader.value.copied = true;
        let opened = reader.value.opened;
        cx.notify();
        cx.spawn(async move |this, cx| {
            cx.background_executor().timer(std::time::Duration::from_millis(1500)).await;
            let _ = this.update(cx, |pane, cx| {
                if let Some(reader) = pane.reader.as_mut().filter(|r| r.value.opened == opened) {
                    reader.value.copied = false;
                    cx.notify();
                }
            });
        })
        .detach();
    }

    /// The reader's file in the editor.
    pub(super) fn edit_reader(&mut self, cx: &mut Context<Self>) {
        let Some((file, cwd)) = self.reader.as_ref().map(|r| (r.value.file.clone(), r.value.cwd.clone())) else { return };
        self.close_reader(cx);
        let Some(PaneQueries(client, rt)) = cx.try_global::<PaneQueries>() else { return };
        let editor = cx.try_global::<crate::settings::SettingsGlobal>().and_then(|s| s.settings.editor.clone());
        let (client, pane_id) = (client.clone(), self.pane_id.clone());
        rt.spawn(async move { edit(&client, &pane_id, &cwd, editor.as_deref(), &file, None, None).await });
    }

    pub(super) fn show_preview(&mut self, path: PathBuf, cx: &mut Context<Self>) {
        // Only the header: the image itself loads as the dialog draws it.
        let size = image::image_dimensions(&path).ok();
        self.preview = Some(Popup::new(Preview { path, size, zoom: 1., pan: (0., 0.), drag: None, opened: std::time::Instant::now() }));
        self.hover = None;
        cx.notify();
    }

    pub(super) fn close_preview(&mut self, cx: &mut Context<Self>) {
        if let Some(preview) = &mut self.preview {
            preview.close();
            cx.notify();
        }
    }

    pub(super) fn zoom_key(&mut self, factor: f32, window: &Window, cx: &mut Context<Self>) {
        if self.preview.as_ref().is_some_and(Popup::is_open) {
            self.zoom_preview(factor, None, window, cx);
        } else {
            cx.propagate();
        }
    }

    pub(super) fn end_preview_drag(&mut self, cx: &mut Context<Self>) {
        if let Some(preview) = self.preview.as_mut().filter(|p| p.value.drag.is_some()) {
            preview.value.drag = None;
            cx.notify();
        }
    }

    /// Scales the open image by `factor` (0: back to the fit), keeping the point under `at` (window coordinates; none: the middle) in place.
    pub(super) fn zoom_preview(&mut self, factor: f32, at: Option<gpui::Point<Pixels>>, window: &Window, cx: &mut Context<Self>) {
        let Some(preview) = self.preview.as_mut().filter(|p| p.is_open()) else { return };
        let frame = PreviewFrame::of(window, 1.);
        let p = &mut preview.value;
        if factor == 0. {
            (p.zoom, p.pan) = (1., (0., 0.));
        } else {
            p.zoom_about(p.zoom * factor, at.map_or((0., 0.), |at| frame.at(at)), &frame);
        }
        cx.notify();
    }

    /// The image dialog, about the window's size: the image fit inside it, scaled by a pinch, Cmd+scroll, Cmd+=/-/0
    /// or the header's buttons, and moved by scrolling or dragging. Zed's image viewer: no bounds, no glides.
    pub(super) fn preview(&self, preview: &Popup<Preview>, theme: &Theme, window: &Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = &preview.value;
        let frame = PreviewFrame::of(window, grown(preview, p.opened, window));
        let name = p.path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
        let (w, h) = p.shown(&frame);
        let percent = format!("{:.0}%", p.scale(&frame) * 100.);
        let card = dialog_card("image-preview", &frame, theme)
            .on_mouse_down_out(cx.listener(|pane, _, _, cx| pane.close_preview(cx)))
            .child(
                dialog_header(name, theme)
                    .child(dialog_button("image-zoom-out", "icons/minus.svg", theme).on_click(cx.listener(|pane, _, window, cx| pane.zoom_preview(1. / ZOOM_STEP, None, window, cx))))
                    .child(
                        div()
                            .id("image-zoom-fit")
                            .flex_none()
                            .w(px(40.))
                            .text_center()
                            .text_size(px(11.))
                            .text_color(theme.text_muted)
                            .cursor_pointer()
                            .child(percent)
                            .on_click(cx.listener(|pane, _, window, cx| pane.zoom_preview(0., None, window, cx))),
                    )
                    .child(dialog_button("image-zoom-in", "icons/plus.svg", theme).on_click(cx.listener(|pane, _, window, cx| pane.zoom_preview(ZOOM_STEP, None, window, cx))))
                    .child(dialog_button("image-preview-close", "icons/close.svg", theme).ml(px(6.)).on_click(cx.listener(|pane, _, _, cx| pane.close_preview(cx)))),
            )
            .child(
                div()
                    .id("image-preview-frame")
                    .relative()
                    .flex_1()
                    .overflow_hidden()
                    .rounded(px(6.))
                    .when(p.drag.is_some(), |el| el.cursor(gpui::CursorStyle::ClosedHand))
                    .on_pinch(cx.listener(|pane, e: &gpui::PinchEvent, window, cx| pane.zoom_preview(1. + e.delta, Some(e.position), window, cx)))
                    .on_scroll_wheel(cx.listener(|pane, e: &ScrollWheelEvent, window, cx| {
                        let d = e.delta.pixel_delta(px(20.));
                        if e.modifiers.platform || e.modifiers.control {
                            let dy = f32::from(d.y);
                            let factor = if dy > 0. { 1. + dy.abs() * 0.01 } else { 1. / (1. + dy.abs() * 0.01) };
                            return pane.zoom_preview(factor, Some(e.position), window, cx);
                        }
                        let Some(preview) = pane.preview.as_mut().filter(|p| p.is_open()) else { return };
                        preview.value.pan_by((f32::from(d.x), f32::from(d.y)), &PreviewFrame::of(window, 1.));
                        cx.notify();
                    }))
                    .on_mouse_down(MouseButton::Left, cx.listener(|pane, e: &MouseDownEvent, _, cx| {
                        if let Some(preview) = pane.preview.as_mut() {
                            preview.value.drag = Some(e.position);
                            cx.notify();
                        }
                    }))
                    .on_mouse_move(cx.listener(|pane, e: &MouseMoveEvent, window, cx| {
                        let Some(p) = pane.preview.as_mut().map(|p| &mut p.value) else { return };
                        // The button's own state, not the mouse up, which can go missing (seen after a Cmd+click).
                        if e.pressed_button != Some(MouseButton::Left) {
                            if p.drag.take().is_some() {
                                cx.notify();
                            }
                            return;
                        }
                        let Some(from) = p.drag.replace(e.position) else { return };
                        p.pan_by((f32::from(e.position.x - from.x), f32::from(e.position.y - from.y)), &PreviewFrame::of(window, 1.));
                        cx.notify();
                    }))
                    .on_mouse_up(MouseButton::Left, cx.listener(|pane, _, _, cx| pane.end_preview_drag(cx)))
                    .on_mouse_up_out(MouseButton::Left, cx.listener(|pane, _, _, cx| pane.end_preview_drag(cx)))
                    // A double click goes between the fit and twice it, about the pointer.
                    .on_click(cx.listener(|pane, e: &gpui::ClickEvent, window, cx| {
                        if e.click_count() == 2 {
                            let zoomed = pane.preview.as_ref().is_some_and(|p| p.value.zoom != 1. || p.value.pan != (0., 0.));
                            pane.zoom_preview(if zoomed { 0. } else { 2. }, Some(e.position()), window, cx);
                        }
                    }))
                    .child(
                        gpui::img(p.path.clone())
                            .absolute()
                            .left(px((frame.area.0 - w) / 2. + p.pan.0))
                            .top(px((frame.area.1 - h) / 2. + p.pan.1))
                            .w(px(w))
                            .h(px(h))
                            .object_fit(gpui::ObjectFit::Contain)
                            .with_fallback(|| div().size_full().flex().items_center().justify_center().child("Can't show this image").into_any_element()),
                    ),
            );
        crate::shell::modal("image-preview", preview, window, card)
    }

    /// The Markdown dialog: the image dialog's card with the file rendered in a column, scrolled.
    pub(super) fn reader(&self, reader: &Popup<Reader>, theme: &Theme, window: &Window, cx: &mut Context<Self>) -> impl IntoElement {
        let r = &reader.value;
        let frame = PreviewFrame::of(window, grown(reader, r.opened, window));
        let name = r.file.path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
        let (images, pane) = (r.images.clone(), cx.entity().downgrade());
        let look = crate::ui::markdown::Look {
            document: true,
            image: Some(std::rc::Rc::new(move |source: &str| images.get(source).cloned())),
            link: Some(std::rc::Rc::new(move |url: &str, _: &mut Window, cx: &mut App| {
                let _ = pane.update(cx, |pane, cx| pane.follow_link(url, cx));
            })),
        };
        let card = dialog_card("markdown-reader", &frame, theme)
            .on_mouse_down_out(cx.listener(|pane, _, _, cx| pane.close_reader(cx)))
            .child(
                dialog_header(name, theme)
                    .child(
                        dialog_button("markdown-reader-copy", if r.copied { "icons/check.svg" } else { "icons/solar/copy.svg" }, theme)
                            .on_click(cx.listener(|pane, _, _, cx| pane.copy_reader(cx))),
                    )
                    .child(
                        div()
                            .id("markdown-reader-edit")
                            .flex_none()
                            .h(px(22.))
                            .px(px(8.))
                            .rounded(px(6.))
                            .flex()
                            .items_center()
                            .text_size(px(11.5))
                            .text_color(theme.text_muted)
                            .cursor_pointer()
                            .hover(|s| s.bg(theme.hover).text_color(theme.text))
                            .child("Open in editor")
                            .on_click(cx.listener(|pane, _, _, cx| pane.edit_reader(cx))),
                    )
                    .child(dialog_button("markdown-reader-close", "icons/close.svg", theme).ml(px(4.)).on_click(cx.listener(|pane, _, _, cx| pane.close_reader(cx)))),
            )
            .child(
                // the text fades where it scrolls under the edges, the app's overlay bar on the right
                div()
                    .relative()
                    .flex_1()
                    .min_h_0()
                    .flex()
                    .flex_col()
                    .child(crate::ui::edge_fade::edge_faded(
                        12.,
                        &r.scroll,
                        div().id("markdown-reader-body").flex_1().min_h_0().overflow_y_scroll().track_scroll(&r.scroll).child(
                            div()
                                .max_w(px(800.))
                                .mx_auto()
                                .px(px(24.))
                                .pt(px(4.))
                                .pb(px(24.))
                                .flex()
                                .flex_col()
                                .gap(px(12.))
                                .text_size(px(14.))
                                .line_height(px(22.))
                                .child(crate::ui::markdown::frame_reset())
                                .children(crate::ui::markdown::render_with(&r.blocks, READER_KEY, theme, &look)),
                        ),
                    ))
                    .child(crate::ui::scrollbar("markdown-reader-scrollbar", &r.scroll, theme)),
            );
        crate::shell::modal("markdown-reader", reader, window, card)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn preview_fits_and_zooms_about_the_pointer() {
        let frame = PreviewFrame { card: (0., 0.), area: (400., 300.), center: (0., 0.) };
        let mut p = Preview {
            path: "a.png".into(),
            size: Some((800, 400)),
            zoom: 1.,
            pan: (0., 0.),
            drag: None,
            opened: std::time::Instant::now(),
        };
        assert_eq!(p.fit(&frame), (400., 200.));
        // The point under the pointer stays: 100 px right of the middle, zoomed 2×, the image moves 100 px left.
        p.zoom_about(2., (100., 0.), &frame);
        assert_eq!((p.zoom, p.pan), (2., (-100., 0.)));
        assert_eq!(p.scale(&frame), 1.);
        // Dragged, it keeps the frame covered: 800×400 over 400×300 overhangs 200 and 50.
        p.pan_by((1000., 1000.), &frame);
        assert_eq!(p.pan, (200., 50.));
        // Smaller than the frame, it stays inside it: at 400×200 it fits exactly across, 50 px of room down.
        p.zoom_about(1., (0., 0.), &frame);
        p.pan_by((-1000., -1000.), &frame);
        assert_eq!(p.pan, (0., -50.));
        p.zoom_about(1000., (0., 0.), &frame);
        assert_eq!(p.zoom, MAX_ZOOM);
        // a small one is not blown up
        assert_eq!(Preview { size: Some((40, 20)), ..p }.fit(&frame), (40., 20.));
    }
}

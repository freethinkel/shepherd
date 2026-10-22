//! Web integrations: local tools' pages listed in the sidebar and shown in place of the tabs and panes. What here is
//! plain logic; the pages themselves are the Shell's (`shell/web.rs`).
use crate::settings::WebEntry;
use gpui::{Bounds, ImageFormat, Pixels, point, px, size};

/// How much of the window's left edge the page leaves while the sidebar is closed: the strip that brings it out.
pub const PEEK_EDGE: f32 = 6.;

/// `localhost:4317` → `http://localhost:4317`; a URL with a scheme is kept; surrounding space trimmed.
pub fn normalize(url: &str) -> String {
    let url = url.trim();
    if url.contains("://") { url.to_string() } else { format!("http://{url}") }
}

/// What an entry goes by in the sidebar and the tab bar: its name, else its page's title once loaded, else the
/// URL's host and port.
pub fn label(entry: &WebEntry, title: Option<&str>) -> String {
    let given = [entry.name.trim(), title.unwrap_or_default().trim()];
    if let Some(name) = given.into_iter().find(|n| !n.is_empty()) {
        return name.to_string();
    }
    let url = normalize(&entry.url);
    url.parse::<http::Uri>().ok().and_then(|u| u.authority().map(|a| a.to_string())).unwrap_or_else(|| entry.url.trim().to_string())
}

/// The entries listed while the place with `place` key is shown, in the settings' order.
pub fn listed<'a>(entries: &'a [WebEntry], place: Option<&str>) -> Vec<&'a WebEntry> {
    entries.iter().filter(|e| e.show_in.is_none() || e.show_in.as_deref() == place).collect()
}

/// Moves entry `id` to where entry `to` is: a row dropped on another takes its place, so it lands after it when
/// moved down and before it when moved up.
pub fn move_entry(entries: &mut Vec<WebEntry>, id: &str, to: &str) {
    let at = |id: &str| entries.iter().position(|e| e.id == id);
    let (Some(from), Some(to)) = (at(id), at(to)) else { return };
    let entry = entries.remove(from);
    entries.insert(to, entry);
}

/// A URL's scheme, lowercased host and port (the scheme's default when none is written); `None` for anything but
/// http and https.
fn origin(url: &str) -> Option<(String, String, u16)> {
    let uri: http::Uri = url.parse().ok()?;
    let scheme = uri.scheme_str()?.to_ascii_lowercase();
    let port = uri.port_u16().unwrap_or(match scheme.as_str() {
        "http" => 80,
        "https" => 443,
        _ => return None,
    });
    Some((scheme, uri.host()?.to_ascii_lowercase(), port))
}

/// Whether `to` leaves `page`'s site: another scheme, host or port. Anything but http and https (about:, data:) is
/// not off-site.
pub fn off_site(page: &str, to: &str) -> bool {
    match (origin(page), origin(to)) {
        (Some(page), Some(to)) => page != to,
        _ => false,
    }
}

/// `name="value"` (or `'value'`) of an HTML tag; the name is matched without regard to case.
fn attribute<'a>(tag: &'a str, name: &str) -> Option<&'a str> {
    let lower = tag.to_ascii_lowercase();
    let mut from = 0;
    while let Some(at) = lower[from..].find(name).map(|i| i + from) {
        from = at + name.len();
        let before = lower[..at].chars().next_back();
        let rest = lower[from..].trim_start();
        if !before.is_some_and(char::is_whitespace) || !rest.starts_with('=') {
            continue;
        }
        let value_at = tag.len() - rest.len() + 1;
        let value = tag[value_at..].trim_start();
        let quote = value.chars().next().filter(|c| *c == '"' || *c == '\'')?;
        return value[1..].split(quote).next();
    }
    None
}

/// The favicon's absolute URL from a page's HTML: the first `<link rel=…icon…>`'s href resolved against `page`, else
/// `<origin>/favicon.ico`.
pub fn favicon_url(html: &str, page: &str) -> String {
    let (scheme, rest) = page.split_once("://").unwrap_or(("http", page));
    let root = format!("{scheme}://{}", rest.split('/').next().unwrap_or(rest));
    let lower = html.to_ascii_lowercase();
    let href = lower.match_indices("<link").find_map(|(at, _)| {
        let tag = &html[at..at + lower[at..].find('>').unwrap_or(lower.len() - at)];
        attribute(tag, "rel").filter(|rel| rel.to_ascii_lowercase().contains("icon"))?;
        attribute(tag, "href").filter(|href| !href.is_empty())
    });
    match href {
        None => format!("{root}/favicon.ico"),
        Some(href) if href.contains("://") => href.to_string(),
        Some(href) if href.starts_with("//") => format!("{scheme}:{href}"),
        Some(href) if href.starts_with('/') => format!("{root}{href}"),
        Some(href) => {
            // relative to the page's folder: its path up to the last `/`
            let path = rest.find('/').map_or("/", |at| &rest[at..]);
            format!("{root}{}{href}", &path[..path.rfind('/').unwrap_or(0) + 1])
        }
    }
}

/// For a forward: the URL's (host, port) to reach on the machine, and the URL to load here through `local_port`
/// (`http://localhost:<local_port>` with the same path and query).
pub fn forwarded(url: &str, local_port: u16) -> Option<((String, u16), String)> {
    let (_, host, port) = origin(url)?;
    let uri: http::Uri = url.parse().ok()?;
    let path = uri.path_and_query().map_or("/", |p| p.as_str());
    Some(((host, port), format!("http://localhost:{local_port}{path}")))
}

/// Whether a live page, made for `url`, still belongs to entry `id`: the entry exists and its normalized URL is `url`.
pub fn page_is_current(entries: &[WebEntry], id: &str, url: &str) -> bool {
    entries.iter().any(|e| e.id == id && normalize(&e.url) == url)
}

/// The web view's rectangle inside the main area: the area itself, less [`PEEK_EDGE`] at its left when the sidebar
/// is closed and the area starts at the window's left edge.
pub fn page_bounds(area: Bounds<Pixels>, sidebar_closed: bool) -> Bounds<Pixels> {
    if !sidebar_closed || area.origin.x >= px(PEEK_EDGE) {
        return area;
    }
    let inset = px(PEEK_EDGE) - area.origin.x;
    Bounds::new(point(area.origin.x + inset, area.origin.y), size(area.size.width - inset, area.size.height))
}

/// A page's favicon: the icon its HTML links to, else its site's `/favicon.ico`, with the image's format. Blocking
/// (5 s at most for each of the two requests): call it off the main thread.
pub fn fetch_favicon(page: &str) -> Option<(ImageFormat, Vec<u8>)> {
    let agent: ureq::Agent = ureq::Agent::config_builder().timeout_global(Some(std::time::Duration::from_secs(5))).build().into();
    let get = |url: &str| agent.get(url).call().ok()?.body_mut().with_config().limit(1 << 20).read_to_vec().ok();
    let html = get(page).map(|b| String::from_utf8_lossy(&b).into_owned()).unwrap_or_default();
    let bytes = get(&favicon_url(&html, page))?;
    Some((image_format(&bytes)?, bytes))
}

/// Whether something replies on local `port`: an SSH forward takes a connection even when nothing listens on the
/// machine, and closes it unanswered. Blocking, 5 s at most.
pub fn answers(port: u16) -> bool {
    use std::io::{Read, Write};
    let Ok(mut stream) = std::net::TcpStream::connect(("127.0.0.1", port)) else { return false };
    let _ = stream.set_read_timeout(Some(std::time::Duration::from_secs(5)));
    stream.write_all(b"HEAD / HTTP/1.0\r\n\r\n").is_ok() && stream.read(&mut [0; 1]).is_ok_and(|n| n > 0)
}

/// An image's format by its first bytes, for GPUI's `Image::from_bytes`; `None` when unknown.
pub fn image_format(bytes: &[u8]) -> Option<ImageFormat> {
    let text = String::from_utf8_lossy(&bytes[..bytes.len().min(256)]).trim_start().to_ascii_lowercase();
    Some(match bytes {
        [0x89, b'P', b'N', b'G', ..] => ImageFormat::Png,
        [0, 0, 1, 0, ..] => ImageFormat::Ico,
        [0xff, 0xd8, 0xff, ..] => ImageFormat::Jpeg,
        [b'G', b'I', b'F', b'8', ..] => ImageFormat::Gif,
        [b'R', b'I', b'F', b'F', _, _, _, _, b'W', b'E', b'B', b'P', ..] => ImageFormat::Webp,
        _ if text.starts_with("<svg") || (text.starts_with("<?xml") && text.contains("<svg")) => ImageFormat::Svg,
        _ => return None,
    })
}

/// The local port a page on a machine is forwarded through, the same each launch for its entry and machine: a page's
/// storage belongs to its origin, port included. In 20000–29999, below the systems' ephemeral ranges.
pub fn forward_port(entry: &str, target: &str) -> u16 {
    // FNV-1a: std's hasher may change between Rust releases
    let hash = format!("{entry}\0{target}").bytes().fold(0xcbf2_9ce4_8422_2325_u64, |h, b| (h ^ u64::from(b)).wrapping_mul(0x100_0000_01b3));
    20000 + (hash % 10000) as u16
}

/// The modifiers held with a key the page was sent.
#[derive(Clone, Copy, Default)]
pub struct Mods {
    pub cmd: bool,
    pub ctrl: bool,
    pub alt: bool,
    pub shift: bool,
}

/// The keystroke of Shepherd's one-key chord among `chords` that a key the page was sent (`characters` as AppKit
/// gives them without modifiers) is, so it goes to Shepherd instead; plain typing is the page's. Shift that only
/// typed the key's symbol (`cmd-{`) may be left out of the chord. Arrows without ⌃ are the page's.
pub fn shepherds_key(mods: Mods, characters: &str, chords: &[String]) -> Option<gpui::Keystroke> {
    if !(mods.cmd || mods.ctrl || mods.alt) {
        return None;
    }
    let key = match characters {
        "\u{F700}" => "up".to_string(),
        "\u{F701}" => "down".to_string(),
        "\u{F702}" => "left".to_string(),
        "\u{F703}" => "right".to_string(),
        "\r" => "enter".to_string(),
        "\t" | "\u{19}" => "tab".to_string(),
        "\u{1b}" => "escape".to_string(),
        "\u{7f}" => "backspace".to_string(),
        " " => "space".to_string(),
        other => other.to_lowercase(),
    };
    // ⌘ and ⌥ with an arrow move the caret and scroll the page; ponytail: always the page's, rebound or not
    if !mods.ctrl && matches!(key.as_str(), "up" | "down" | "left" | "right") {
        return None;
    }
    let combo = |shift: bool| {
        let held = [(mods.cmd, "cmd-"), (mods.ctrl, "ctrl-"), (mods.alt, "alt-"), (shift, "shift-")];
        let prefix: String = held.iter().filter(|(on, _)| *on).map(|(_, m)| *m).collect();
        gpui::Keystroke::parse(&format!("{prefix}{key}")).ok()
    };
    let pressed = [combo(mods.shift), combo(false).filter(|_| mods.shift)];
    chords
        .iter()
        .filter(|c| !c.trim().contains(' '))
        .filter_map(|c| gpui::Keystroke::parse(c.trim()).ok())
        .find(|k| pressed.iter().flatten().any(|p| p == k))
}

/// The edit action (AppKit's selector) of a standard editing chord pressed in a page: WebKit takes these from the
/// Edit menu, which Shepherd hasn't, so the page's key monitor sends them itself.
pub fn edit_action(mods: Mods, characters: &str) -> Option<&'static std::ffi::CStr> {
    if !mods.cmd || mods.ctrl {
        return None;
    }
    Some(match (characters.to_lowercase().as_str(), mods.alt, mods.shift) {
        ("c", false, false) => c"copy:",
        ("x", false, false) => c"cut:",
        ("v", false, false) => c"paste:",
        ("v", true, true) => c"pasteAsPlainText:",
        ("a", false, false) => c"selectAll:",
        ("z", false, false) => c"undo:",
        ("z", false, true) => c"redo:",
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::settings::WebEntry;

    #[test]
    fn an_entry_without_a_name_goes_by_its_pages_title_else_its_host() {
        let e = |name: &str| WebEntry { name: name.into(), url: "localhost:4317/board".into(), ..Default::default() };
        assert_eq!(label(&e("board"), Some("herdr board")), "board");
        assert_eq!(label(&e(" "), Some("herdr board")), "herdr board");
        assert_eq!(label(&e(""), None), "localhost:4317");
        assert_eq!(label(&e(""), Some(" ")), "localhost:4317");
        let blank = WebEntry::default();
        assert_eq!(label(&blank, None), "");
    }

    #[test]
    fn a_url_without_a_scheme_is_http() {
        assert_eq!(normalize(" localhost:4317 "), "http://localhost:4317");
        assert_eq!(normalize("127.0.0.1:8080/board"), "http://127.0.0.1:8080/board");
        assert_eq!(normalize("https://x.dev/a"), "https://x.dev/a");
    }

    #[test]
    fn an_entry_tied_to_a_place_lists_only_there() {
        let e = |id: &str, show_in: Option<&str>| WebEntry { id: id.into(), show_in: show_in.map(Into::into), ..Default::default() };
        let entries = [e("a", None), e("b", Some("machine:m1")), e("c", Some("/tmp/s.sock"))];
        let ids = |p| listed(&entries, p).iter().map(|e| e.id.as_str()).collect::<Vec<_>>();
        assert_eq!(ids(Some("machine:m1")), ["a", "b"]);
        assert_eq!(ids(Some("/tmp/s.sock")), ["a", "c"]);
        assert_eq!(ids(None), ["a"]);
    }

    #[test]
    fn off_site_is_another_scheme_host_or_port() {
        let page = "http://localhost:4317/board";
        assert!(!off_site(page, "http://localhost:4317/other?q=1"));
        assert!(off_site(page, "http://localhost:3000/"));
        assert!(off_site(page, "https://localhost:4317/"));
        assert!(off_site(page, "https://github.com/x"));
        assert!(!off_site(page, "about:blank"));
    }

    #[test]
    fn the_favicon_is_the_pages_icon_link_else_favicon_ico() {
        let page = "http://localhost:4317/board/";
        assert_eq!(favicon_url(r#"<head><link rel="icon" href="/static/i.png"></head>"#, page), "http://localhost:4317/static/i.png");
        assert_eq!(favicon_url(r#"<LINK href='i.svg' rel='shortcut icon'>"#, page), "http://localhost:4317/board/i.svg");
        assert_eq!(favicon_url(r#"<link rel="icon" href="https://cdn.x/i.ico">"#, page), "https://cdn.x/i.ico");
        assert_eq!(favicon_url(r#"<link rel="icon" href="//cdn.x/i.ico">"#, page), "http://cdn.x/i.ico");
        assert_eq!(favicon_url(r#"<link rel="stylesheet" href="a.css">"#, page), "http://localhost:4317/favicon.ico");
        assert_eq!(favicon_url("<html></html>", page), "http://localhost:4317/favicon.ico");
    }

    #[test]
    fn a_forwarded_url_keeps_its_path_on_the_local_port() {
        assert_eq!(
            forwarded("http://localhost:4317/board?x=1", 50123),
            Some((("localhost".into(), 4317), "http://localhost:50123/board?x=1".into()))
        );
        assert_eq!(forwarded("http://10.0.0.5/", 50123), Some((("10.0.0.5".into(), 80), "http://localhost:50123/".into())));
        assert_eq!(forwarded("https://10.0.0.5", 50123), Some((("10.0.0.5".into(), 443), "http://localhost:50123/".into())));
    }

    #[test]
    fn a_page_is_current_only_for_its_entrys_url() {
        let entries = [WebEntry { id: "a".into(), url: "localhost:1".into(), ..Default::default() }];
        assert!(page_is_current(&entries, "a", "http://localhost:1"));
        assert!(!page_is_current(&entries, "a", "http://localhost:2"));
        assert!(!page_is_current(&entries, "gone", "http://localhost:1"));
    }

    #[test]
    fn the_page_leaves_the_peek_edge_while_the_sidebar_is_closed() {
        use gpui::{Bounds, point, px, size};
        let area = Bounds::new(point(px(0.), px(38.)), size(px(800.), px(500.)));
        assert_eq!(page_bounds(area, false), area);
        assert_eq!(page_bounds(area, true), Bounds::new(point(px(6.), px(38.)), size(px(794.), px(500.))));
        let docked = Bounds::new(point(px(256.), px(38.)), size(px(600.), px(500.)));
        assert_eq!(page_bounds(docked, true), docked);
    }

    #[test]
    fn a_favicon_is_fetched_from_the_pages_icon_link() {
        use std::io::{Read, Write};
        let server = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let port = server.local_addr().unwrap().port();
        std::thread::spawn(move || {
            for stream in server.incoming().take(2) {
                let mut stream = stream.unwrap();
                let mut request = [0; 1024];
                let n = stream.read(&mut request).unwrap();
                let body: &[u8] = match request[..n].starts_with(b"GET /i.png") {
                    true => b"\x89PNG\r\n\x1a\nrest",
                    false => br#"<link rel="icon" href="/i.png">"#,
                };
                write!(stream, "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n", body.len()).unwrap();
                stream.write_all(body).unwrap();
            }
        });
        let (format, bytes) = fetch_favicon(&format!("http://127.0.0.1:{port}/board")).unwrap();
        assert_eq!(format, gpui::ImageFormat::Png);
        assert!(bytes.starts_with(b"\x89PNG"));
    }

    #[test]
    fn a_port_answers_only_when_something_replies() {
        use std::io::{Read, Write};
        let serve = |reply: bool| {
            let server = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
            let port = server.local_addr().unwrap().port();
            std::thread::spawn(move || {
                let (mut stream, _) = server.accept().unwrap();
                let _ = stream.read(&mut [0; 256]);
                if reply {
                    let _ = stream.write_all(b"HTTP/1.0 200 OK\r\n\r\n");
                }
            });
            port
        };
        assert!(answers(serve(true)));
        // as an SSH forward does when nothing listens on the machine: it takes the connection, then closes it
        assert!(!answers(serve(false)));
    }

    #[test]
    fn nothing_listening_no_favicon() {
        let port = std::net::TcpListener::bind("127.0.0.1:0").unwrap().local_addr().unwrap().port();
        assert!(fetch_favicon(&format!("http://127.0.0.1:{port}/")).is_none());
    }

    #[test]
    fn favicon_formats_by_magic_bytes() {
        use gpui::ImageFormat;
        assert_eq!(image_format(b"\x89PNG\r\n\x1a\n...."), Some(ImageFormat::Png));
        assert_eq!(image_format(&[0, 0, 1, 0, 1, 0]), Some(ImageFormat::Ico));
        assert_eq!(image_format(b"<svg xmlns=..."), Some(ImageFormat::Svg));
        assert_eq!(image_format(b"<?xml version=\"1.0\"?><svg"), Some(ImageFormat::Svg));
        assert_eq!(image_format(&[0xff, 0xd8, 0xff, 0xe0]), Some(ImageFormat::Jpeg));
        assert_eq!(image_format(b"GIF89a"), Some(ImageFormat::Gif));
        assert_eq!(image_format(b"RIFF\0\0\0\0WEBPVP8 "), Some(ImageFormat::Webp));
        assert_eq!(image_format(b"nope"), None);
    }

    #[test]
    fn a_dropped_entry_takes_the_place_of_the_one_it_is_dropped_on() {
        let mut entries: Vec<WebEntry> = ["a", "b", "c"].map(|id| WebEntry { id: id.into(), ..Default::default() }).into();
        let ids = |e: &[WebEntry]| e.iter().map(|e| e.id.clone()).collect::<Vec<_>>().join("");
        move_entry(&mut entries, "a", "c");
        assert_eq!(ids(&entries), "bca");
        move_entry(&mut entries, "a", "b");
        assert_eq!(ids(&entries), "abc");
        move_entry(&mut entries, "a", "gone");
        assert_eq!(ids(&entries), "abc");
    }

    #[test]
    fn a_standard_editing_chord_in_a_page_is_its_edit_action() {
        let cmd = Mods { cmd: true, ..Mods::default() };
        assert_eq!(edit_action(cmd, "c"), Some(c"copy:"));
        assert_eq!(edit_action(cmd, "v"), Some(c"paste:"));
        assert_eq!(edit_action(Mods { shift: true, ..cmd }, "Z"), Some(c"redo:"));
        assert_eq!(edit_action(Mods { ctrl: true, ..cmd }, "c"), None);
        assert_eq!(edit_action(Mods::default(), "c"), None);
        assert_eq!(edit_action(cmd, "k"), None);
    }

    #[test]
    fn a_key_the_page_has_is_shepherds_when_shepherd_binds_it() {
        let chords = ["cmd-k", "cmd-shift-w", "cmd-{", "cmd-up", "ctrl-a x"].map(String::from);
        let key = |mods, chars| shepherds_key(mods, chars, &chords);
        let ks = |k| gpui::Keystroke::parse(k).ok();
        let (cmd, shift) = (Mods { cmd: true, ..Mods::default() }, Mods { cmd: true, shift: true, ..Mods::default() });
        assert_eq!(key(cmd, "k"), ks("cmd-k"));
        assert_eq!(key(shift, "W"), ks("cmd-shift-w"));
        // shift typed the brace: the chord names the brace alone
        assert_eq!(key(shift, "{"), ks("cmd-{"));
        assert_eq!(key(cmd, "c"), None);
        assert_eq!(key(Mods::default(), "k"), None);
        // arrows move the caret and scroll in a page: ⌘↑ stays the page's though Shepherd binds it
        assert_eq!(key(cmd, "\u{F700}"), None);
        assert_eq!(key(Mods { cmd: true, alt: true, ..Mods::default() }, "\u{F702}"), None);
        // a chord of two keys isn't taken from the page
        assert_eq!(key(Mods { ctrl: true, ..Mods::default() }, "a"), None);
    }

    #[test]
    fn a_page_on_a_machine_keeps_its_port_between_launches() {
        let port = forward_port("entry-1", "stand");
        assert_eq!(port, forward_port("entry-1", "stand"));
        assert_ne!(port, forward_port("entry-1", "other"));
        assert_ne!(port, forward_port("entry-2", "stand"));
        assert!((20000..30000).contains(&port));
    }
}

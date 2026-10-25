//! herdr's client-shell input: gpui's keystrokes as herdr's semantic keys, which herdr encodes for the pane app's
//! keyboard protocol. The connection that carries them is the popup watch ([`super::popup`]).
use super::wire::{KeyCode, KeyKind, PaneInput};

pub use super::wire::{DefaultColorKind, HostAppearance, HostThemeUpdate, Rgb};

/// gpui's keystroke as herdr's semantic key; `None` for the platform chords that drive Shepherd itself.
pub fn key_event(key: &str, key_char: Option<&str>, mods: &gpui::Modifiers) -> Option<PaneInput> {
    if mods.platform {
        // Cmd+Backspace: delete to the line's start (Ctrl+U), Cmd+Left/Right: to its start or end (Ctrl+A, Ctrl+E),
        // as in the attach path
        let text = match key {
            "backspace" => "\x15",
            "left" => "\x01",
            "right" => "\x05",
            _ => return None,
        };
        return Some(PaneInput::TextCommit(text.into()));
    }
    // Option+Left/Right: a word back or forward (ESC b, ESC f), as Ghostty sends them
    if mods.alt && !mods.shift && !mods.control && matches!(key, "left" | "right") {
        return Some(PaneInput::TextCommit(if key == "left" { "\x1bb" } else { "\x1bf" }.into()));
    }
    let code = match key {
        "enter" => KeyCode::Enter,
        "backspace" => KeyCode::Backspace,
        "tab" if mods.shift => KeyCode::BackTab,
        "tab" => KeyCode::Tab,
        "escape" => KeyCode::Esc,
        "space" => KeyCode::Char(' '),
        "up" => KeyCode::Up,
        "down" => KeyCode::Down,
        "left" => KeyCode::Left,
        "right" => KeyCode::Right,
        "home" => KeyCode::Home,
        "end" => KeyCode::End,
        "insert" => KeyCode::Insert,
        "delete" => KeyCode::Delete,
        "pageup" => KeyCode::PageUp,
        "pagedown" => KeyCode::PageDown,
        f if f.len() > 1 && f.starts_with('f') && f[1..].parse::<u8>().is_ok() => KeyCode::F(f[1..].parse().ok()?),
        _ => {
            // Option chords macOS composed (Option+B → "∫") go as the text, as Ghostty does by default.
            let text = key_char.filter(|c| !c.is_empty());
            if mods.alt
                && !mods.control
                && let Some(text) = text.filter(|t| *t != key)
            {
                return Some(PaneInput::TextCommit(text.into()));
            }
            let mut chars = key.chars();
            match (chars.next(), chars.next()) {
                (Some(c), None) => KeyCode::Char(c),
                _ => return text.map(|t| PaneInput::TextCommit(t.into())),
            }
        }
    };
    let modifiers = mods.shift as u8 | (mods.control as u8) << 1 | (mods.alt as u8) << 2;
    let generated_text = key_char.filter(|c| !c.is_empty() && !mods.control && !mods.alt).map(str::to_string);
    Some(PaneInput::Key {
        code,
        modifiers,
        kind: KeyKind::Press,
        repeat_count: 1,
        shifted_codepoint: None,
        generated_text,
        tracks_release: false,
        physical_key_id: None,
        windows_record: None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keys_go_semantic_and_composed_text_as_text() {
        let none = gpui::Modifiers::default();
        let ctrl = gpui::Modifiers { control: true, ..Default::default() };
        let alt = gpui::Modifiers { alt: true, ..Default::default() };
        assert!(matches!(key_event("c", Some("c"), &ctrl), Some(PaneInput::Key { code: KeyCode::Char('c'), modifiers: 2, generated_text: None, .. })));
        assert!(matches!(key_event("enter", None, &none), Some(PaneInput::Key { code: KeyCode::Enter, .. })));
        assert!(matches!(key_event("b", Some("∫"), &alt), Some(PaneInput::TextCommit(t)) if t == "∫"));
        // as Ghostty sends them, not as the kitty or legacy encoding of an arrow would
        assert!(matches!(key_event("left", None, &alt), Some(PaneInput::TextCommit(t)) if t == "\x1bb"));
        let cmd = gpui::Modifiers { platform: true, ..Default::default() };
        assert!(matches!(key_event("right", None, &cmd), Some(PaneInput::TextCommit(t)) if t == "\x05"));
        assert!(key_event("j", Some("j"), &cmd).is_none());
        assert!(key_event("q", Some("q"), &gpui::Modifiers { platform: true, ..Default::default() }).is_none());
    }
}

/// Keystroke → changed screen on one attach, the key sent over the attach (`Input`) or over a client shell
/// (`ClientShellPaneInput`): `cargo test --lib client_shell::input_paths -- --ignored --nocapture`.
#[cfg(test)]
mod input_paths {
    use super::*;
    use crate::herdr::session::{Mode, Session, SessionEvent, client_socket};
    use crate::herdr::socket;
    use crate::herdr::wire::{self, ClientMessage, ServerMessage};
    use futures::StreamExt;
    use futures::channel::mpsc::UnboundedReceiver;
    use std::sync::Mutex;
    use std::time::Duration;

    #[test]
    #[ignore]
    fn attach_echo_latency() {
        let mut stream = socket::connect(&client_socket()).unwrap();
        let hello = serde_json::json!({
            "generation": 1, "cell_width_px": 0, "cell_height_px": 0, "surface_size": {"cols": 80, "rows": 24},
            "pixel_mouse": false, "direct_graphics": false, "endpoint_keybindings": false, "mouse_capture": false,
            "surface_active": true, "snapshot_codecs": ["shell.snapshot.v1"], "surface_codecs": ["shell.surface.v1"],
            "input_codecs": ["shell.input.semantic.v1"], "blob_codecs": ["shell.blob.v1"],
        });
        wire::write_message(&mut stream, &ClientMessage::EndpointControl { kind: "endpoint.hello.v1".into(), data: hello.to_string() }).unwrap();
        let pane = loop {
            if let Ok(Some(ServerMessage::PaneSurface(s))) = wire::read_message(&mut stream) {
                break s.panes.iter().find(|p| p.focused).unwrap_or(&s.panes[0]).pane_id.clone();
            }
        };
        let mut reader = stream.try_clone().unwrap();
        std::thread::spawn(move || while wire::read_message(&mut reader).is_ok() {});
        let shell = Mutex::new(stream);
        let (session, mut frames) = Session::spawn(&pane, 80, 24, (0, 0), Mode::Control).unwrap();
        let mut emu = crate::terminal::emulator::Emulator::new(80, 24);
        // `RUN`: a command typed into the pane first, e.g. a program that echoes each key after a delay
        if let Ok(run) = std::env::var("RUN") {
            session.input(format!("{run}\r").as_bytes());
            std::thread::sleep(Duration::from_secs(1));
            drain(&mut frames, &mut emu);
        }
        // what Shepherd types through: the popup watch, once its surface names the pane
        let (_events, watch) = super::super::popup::watch(80, 24, (0, 0)).unwrap();
        std::thread::sleep(Duration::from_millis(300));
        println!("pane {pane}");
        for _ in 0..2 {
            bench("attach Input", &mut frames, &mut emu, |c| session.input(c.as_bytes()));
            bench("popup::Surface::type_into", &mut frames, &mut emu, |c| assert!(watch.type_into(&pane, c.as_bytes())));
            bench("ClientShellPaneInput", &mut frames, &mut emu, |c| {
                let input = ClientMessage::ClientShellPaneInput { pane_id: pane.clone(), events: vec![PaneInput::TextCommit(c.into())] };
                wire::write_message(&mut *shell.lock().unwrap(), &input).unwrap();
            });
        }
        session.input(b"\x03");
    }

    /// The wheel over `PANE`, sent over the attach (`AttachScroll`) or over a client shell (a wheel `Mouse`):
    /// `PANE=… cargo test --lib client_shell::input_paths::wheel -- --ignored --nocapture`. Over history both take
    /// under a millisecond; over an app reading the mouse (`RUN='seq 1 2000 | less --mouse' DOWN_FIRST=1`) the attach
    /// waits out its cadence: ~20 ms against ~0.8 ms.
    #[test]
    #[ignore]
    fn wheel_scroll_latency() {
        let (_events, watch) = super::super::popup::watch(80, 24, (0, 0)).unwrap();
        let pane = std::env::var("PANE").expect("PANE: a pane of herdr's focused tab");
        let (session, mut frames) = Session::spawn(&pane, 80, 24, (0, 0), Mode::Control).unwrap();
        let mut emu = crate::terminal::emulator::Emulator::new(80, 24);
        session.input(std::env::var("RUN").unwrap_or("clear; seq 1 2000".into()).as_bytes());
        session.input(b"\r");
        std::thread::sleep(Duration::from_secs(1));
        drain(&mut frames, &mut emu);
        println!("pane {pane}");
        for _ in 0..2 {
            scroll_bench("attach AttachScroll", &mut frames, &mut emu, |up| session.scroll(up, 3, Some((10, 10))));
            scroll_bench("ClientShellPaneInput Mouse", &mut frames, &mut emu, |up| {
                assert!(watch.pane_input(&pane, vec![PaneInput::wheel(up, 3, 10, 10)]));
            });
        }
    }

    /// A click over a client shell lands on the pane's own cell, not the shell surface's: run a program printing the
    /// mouse reports it gets in `PANE` (not the tab's leftmost), then
    /// `cargo test --lib client_shell::input_paths::click -- --ignored --nocapture` and read the pane.
    #[test]
    #[ignore]
    fn click_lands_on_the_panes_cell() {
        let (_events, watch) = super::super::popup::watch(80, 24, (0, 0)).unwrap();
        let pane = std::env::var("PANE").expect("PANE: a pane of herdr's focused tab");
        std::thread::sleep(Duration::from_millis(300));
        assert!(watch.pane_input(&pane, vec![PaneInput::mouse("down", "left", 3, 2, 0), PaneInput::mouse("up", "left", 3, 2, 0)]));
        std::thread::sleep(Duration::from_millis(300));
    }

    /// A grid resize over the attach until herdr's first frame at the new size and until frames stop (50 ms quiet),
    /// with `RUN` in `PANE`: `PANE=… RUN=vim cargo test --lib client_shell::input_paths::resize -- --ignored --nocapture`.
    #[test]
    #[ignore]
    fn resize_latency() {
        let pane = std::env::var("PANE").expect("PANE: a pane of herdr's");
        let (session, mut frames) = Session::spawn(&pane, 80, 24, (16, 32), Mode::Control).unwrap();
        if let Ok(run) = std::env::var("RUN") {
            session.input(format!("{run}\r").as_bytes());
        }
        std::thread::sleep(Duration::from_secs(1));
        while frames.try_recv().is_ok() {}
        let (mut first, mut last) = (Vec::new(), Vec::new());
        for i in 0..30 {
            let start = std::time::Instant::now();
            session.resize(if i % 2 == 0 { 70 } else { 80 }, 24, (16, 32));
            let _ = futures::executor::block_on(frames.next());
            first.push(start.elapsed().as_secs_f64() * 1000.0);
            let mut end = start.elapsed();
            loop {
                std::thread::sleep(Duration::from_millis(50));
                let mut more = false;
                while frames.try_recv().is_ok() {
                    more = true;
                }
                if !more {
                    break;
                }
                end = start.elapsed() - Duration::from_millis(25);
            }
            last.push(end.as_secs_f64() * 1000.0);
        }
        for (name, mut took) in [("first frame", first), ("frames stop (±25 ms)", last)] {
            took.sort_by(f64::total_cmp);
            println!("resize → {name}, ms: p50 {:.2}, p90 {:.2}, max {:.2}", took[took.len() / 2], took[took.len() * 9 / 10], took[took.len() - 1]);
        }
    }

    fn scroll_bench(name: &str, frames: &mut UnboundedReceiver<SessionEvent>, emu: &mut crate::terminal::emulator::Emulator, send: impl Fn(bool)) {
        let mut took = Vec::new();
        for i in 0..40 {
            std::thread::sleep(Duration::from_millis(50));
            drain(frames, emu);
            let before = emu.row_text(0);
            let start = std::time::Instant::now();
            send((i % 4 < 2) != std::env::var_os("DOWN_FIRST").is_some()); // twice one way, twice back: never past an end
            while emu.row_text(0) == before {
                match futures::executor::block_on(frames.next()) {
                    Some(SessionEvent::Frame(bytes)) => drop(emu.feed(&bytes)),
                    Some(_) => {}
                    None => return,
                }
            }
            took.push(start.elapsed().as_secs_f64() * 1000.0);
        }
        took.sort_by(f64::total_cmp);
        println!("{name}: wheel → scrolled screen, ms: p50 {:.2}, p90 {:.2}, max {:.2}", took[took.len() / 2], took[took.len() * 9 / 10], took[took.len() - 1]);
    }

    fn drain(frames: &mut UnboundedReceiver<SessionEvent>, emu: &mut crate::terminal::emulator::Emulator) {
        while let Ok(event) = frames.try_recv() {
            if let SessionEvent::Frame(bytes) = event {
                emu.feed(&bytes);
            }
        }
    }

    fn bench(
        name: &str,
        frames: &mut UnboundedReceiver<SessionEvent>,
        emu: &mut crate::terminal::emulator::Emulator,
        send: impl Fn(&str),
    ) {
        let screen = |emu: &crate::terminal::emulator::Emulator| (0..emu.rows()).map(|r| emu.row_text(r)).collect::<Vec<_>>();
        let mut took = Vec::new();
        for i in 0..40 {
            std::thread::sleep(Duration::from_millis(50));
            drain(frames, emu);
            let before = screen(emu);
            let start = std::time::Instant::now();
            send(["q", "w", "e", "r"][i % 4]);
            while screen(emu) == before {
                match futures::executor::block_on(frames.next()) {
                    Some(SessionEvent::Frame(bytes)) => drop(emu.feed(&bytes)),
                    Some(_) => {}
                    None => return,
                }
            }
            took.push(start.elapsed().as_secs_f64() * 1000.0);
        }
        send("\x15"); // Ctrl+U: clear the typed line
        std::thread::sleep(Duration::from_millis(100));
        took.sort_by(f64::total_cmp);
        println!("{name}: keystroke → changed screen, ms: p50 {:.2}, p90 {:.2}, max {:.2}", took[took.len() / 2], took[took.len() * 9 / 10], took[took.len() - 1]);
    }
}

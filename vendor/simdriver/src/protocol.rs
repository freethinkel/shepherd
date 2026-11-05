//! JSON-lines wire protocol between the Rust core and the native helpers.

use serde_json::Value;

/// Touch phase. Public API.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Phase {
    Down,
    Move,
    Up,
}

impl Phase {
    fn wire(self) -> &'static str {
        match self {
            Phase::Down => "down",
            Phase::Move => "move",
            Phase::Up => "up",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Edge {
    None,
    Top,
    Left,
    Bottom,
    Right,
}

impl Edge {
    fn code(self) -> u8 {
        match self {
            Edge::None => 0,
            Edge::Top => 1,
            Edge::Left => 2,
            Edge::Bottom => 3,
            Edge::Right => 4,
        }
    }
}

/// Messages from the frame helper (stdout).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FrameMsg {
    Shm { name: String, slots: u32, cap: u64 },
    Frame { slot: u32, seq: u64 },
    Note(String),
}

pub fn parse_frame_line(line: &str) -> Option<FrameMsg> {
    let v: Value = serde_json::from_str(line.trim()).ok()?;
    match v.get("t")?.as_str()? {
        "shm" => Some(FrameMsg::Shm {
            name: v.get("name")?.as_str()?.to_string(),
            slots: u32::try_from(v.get("slots")?.as_u64()?).ok()?,
            cap: v.get("cap")?.as_u64()?,
        }),
        "frame" => Some(FrameMsg::Frame {
            slot: u32::try_from(v.get("slot")?.as_u64()?).ok()?,
            seq: v.get("seq")?.as_u64()?,
        }),
        "note" => Some(FrameMsg::Note(v.get("detail")?.as_str()?.to_string())),
        _ => None,
    }
}

/// Messages from the HID helper (stdout).
#[derive(Debug, Clone, PartialEq)]
pub enum HidMsg {
    Ready { w: f64, h: f64 },
    Warn(String),
    Error(String),
    Displaced,
    Reclaimed,
}

pub fn parse_hid_line(line: &str) -> Option<HidMsg> {
    let v: Value = serde_json::from_str(line.trim()).ok()?;
    let detail = || {
        v.get("detail")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string()
    };
    match v.get("t")?.as_str()? {
        "ready" => Some(HidMsg::Ready {
            w: v.get("w")?.as_f64()?,
            h: v.get("h")?.as_f64()?,
        }),
        "warn" => Some(HidMsg::Warn(detail())),
        "error" => Some(HidMsg::Error(detail())),
        "displaced" => Some(HidMsg::Displaced),
        "reclaimed" => Some(HidMsg::Reclaimed),
        _ => None,
    }
}

/// Commands to the HID helper (stdin).
#[derive(Debug, Clone, PartialEq)]
pub enum HidCmd {
    Touch {
        phase: Phase,
        x: f64,
        y: f64,
        edge: Edge,
    },
    Touch2 {
        phase: Phase,
        x1: f64,
        y1: f64,
        x2: f64,
        y2: f64,
    },
    Button {
        name: &'static str,
        presses: u32,
        hold_secs: f32,
    },
    Key {
        code: u32,
    },
    KeyCombo {
        code: u32,
        mods: Vec<u32>,
    },
    Text(String),
}

impl HidCmd {
    /// One JSON line (no trailing newline) in sdhid's stdin format.
    pub fn to_line(&self) -> String {
        match self {
            HidCmd::Touch { phase, x, y, edge } => format!(
                r#"{{"t":"touch","p":"{}","x":{:?},"y":{:?},"e":{}}}"#,
                phase.wire(),
                x,
                y,
                edge.code()
            ),
            HidCmd::Touch2 {
                phase,
                x1,
                y1,
                x2,
                y2,
            } => format!(
                r#"{{"t":"touch2","p":"{}","x1":{:?},"y1":{:?},"x2":{:?},"y2":{:?}}}"#,
                phase.wire(),
                x1,
                y1,
                x2,
                y2
            ),
            HidCmd::Button {
                name,
                presses,
                hold_secs,
            } => {
                let mut s = format!(r#"{{"t":"button","b":"{name}""#);
                if *presses > 1 {
                    s.push_str(&format!(r#","n":{presses}"#));
                }
                if *hold_secs > 0.0 {
                    s.push_str(&format!(r#","d":{hold_secs:?}"#));
                }
                s.push('}');
                s
            }
            HidCmd::Key { code } => format!(r#"{{"t":"key","code":{code}}}"#),
            HidCmd::KeyCombo { code, mods } => format!(
                r#"{{"t":"key","code":{code},"mods":{}}}"#,
                serde_json::to_string(mods).expect("modifier list serializes")
            ),
            HidCmd::Text(t) => format!(
                r#"{{"t":"text","s":{}}}"#,
                serde_json::to_string(t).expect("string serializes")
            ),
        }
    }

    /// True for commands that begin a new gesture (as opposed to continuing one).
    pub fn is_gesture_start(&self) -> bool {
        match self {
            HidCmd::Touch { phase, .. } | HidCmd::Touch2 { phase, .. } => *phase == Phase::Down,
            HidCmd::Button { .. }
            | HidCmd::Key { .. }
            | HidCmd::KeyCombo { .. }
            | HidCmd::Text(_) => true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frame_lines_parse() {
        assert_eq!(
            parse_frame_line(r#"{"t":"shm","name":"/sdf-123-1","slots":3,"cap":3162112}"#),
            Some(FrameMsg::Shm {
                name: "/sdf-123-1".into(),
                slots: 3,
                cap: 3162112
            })
        );
        assert_eq!(
            parse_frame_line(r#"{"t":"frame","slot":0,"seq":42}"#),
            Some(FrameMsg::Frame { slot: 0, seq: 42 })
        );
        assert_eq!(
            parse_frame_line(r#"{"t":"note","detail":"hi"}"#),
            Some(FrameMsg::Note("hi".into()))
        );
    }

    #[test]
    fn frame_garbage_is_none() {
        for l in [
            "",
            "nope",
            "{}",
            r#"{"t":"x"}"#,
            r#"{"t":"frame","slot":"a"}"#,
            r#"{"t":"shm","name":"/a"}"#,
        ] {
            assert_eq!(parse_frame_line(l), None, "{l}");
        }
    }

    #[test]
    fn hid_lines_parse() {
        assert_eq!(
            parse_hid_line(r#"{"t":"ready","w":393,"h":852}"#),
            Some(HidMsg::Ready { w: 393.0, h: 852.0 })
        );
        assert_eq!(
            parse_hid_line(r#"{"t":"warn","detail":"w"}"#),
            Some(HidMsg::Warn("w".into()))
        );
        assert_eq!(
            parse_hid_line(r#"{"t":"error","detail":"e"}"#),
            Some(HidMsg::Error("e".into()))
        );
        assert_eq!(
            parse_hid_line(r#"{"t":"displaced"}"#),
            Some(HidMsg::Displaced)
        );
        assert_eq!(
            parse_hid_line(r#"{"t":"reclaimed"}"#),
            Some(HidMsg::Reclaimed)
        );
        assert_eq!(parse_hid_line("garbage"), None);
    }

    #[test]
    fn cmd_lines() {
        assert_eq!(
            HidCmd::Touch {
                phase: Phase::Down,
                x: 10.0,
                y: 20.0,
                edge: Edge::Bottom
            }
            .to_line(),
            r#"{"t":"touch","p":"down","x":10.0,"y":20.0,"e":3}"#
        );
        assert_eq!(
            HidCmd::Button {
                name: "HOME",
                presses: 2,
                hold_secs: 0.0
            }
            .to_line(),
            r#"{"t":"button","b":"HOME","n":2}"#
        );
        assert_eq!(
            HidCmd::Button {
                name: "LOCK",
                presses: 1,
                hold_secs: 1.5
            }
            .to_line(),
            r#"{"t":"button","b":"LOCK","d":1.5}"#
        );
        assert_eq!(
            HidCmd::Touch2 {
                phase: Phase::Move,
                x1: 1.0,
                y1: 2.0,
                x2: 3.5,
                y2: 4.0
            }
            .to_line(),
            r#"{"t":"touch2","p":"move","x1":1.0,"y1":2.0,"x2":3.5,"y2":4.0}"#
        );
        assert_eq!(
            HidCmd::Key { code: 40 }.to_line(),
            r#"{"t":"key","code":40}"#
        );
        assert_eq!(
            HidCmd::KeyCombo {
                code: 4,
                mods: vec![227]
            }
            .to_line(),
            r#"{"t":"key","code":4,"mods":[227]}"#
        );
        assert_eq!(
            HidCmd::Text("a\"b".into()).to_line(),
            r#"{"t":"text","s":"a\"b"}"#
        );
    }

    #[test]
    fn gesture_start() {
        let t = |p| HidCmd::Touch {
            phase: p,
            x: 0.0,
            y: 0.0,
            edge: Edge::None,
        };
        let t2 = |p| HidCmd::Touch2 {
            phase: p,
            x1: 0.0,
            y1: 0.0,
            x2: 0.0,
            y2: 0.0,
        };
        assert!(t(Phase::Down).is_gesture_start());
        assert!(!t(Phase::Move).is_gesture_start());
        assert!(!t(Phase::Up).is_gesture_start());
        assert!(t2(Phase::Down).is_gesture_start());
        assert!(!t2(Phase::Up).is_gesture_start());
        assert!(
            HidCmd::Button {
                name: "HOME",
                presses: 1,
                hold_secs: 0.0
            }
            .is_gesture_start()
        );
        assert!(HidCmd::Key { code: 4 }.is_gesture_start());
        assert!(HidCmd::Text(String::new()).is_gesture_start());
    }
}

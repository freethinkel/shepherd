//! `SHEPHERD_MOCK=<dir>`: the window from a fixture instead of herdr, for screenshots (`docs/mock`). The folder holds
//! `snapshot.json` (herdr's `session.snapshot` result with `$HOME` expanded, plus `waited`: pane id → seconds its
//! agent has waited), `panes/<pane id>.txt` (what each pane shows, `\e` for ESC) and `bin/` (a `gh` and a `herdr`
//! answering from files). Nothing else changes: git runs for real in the fixture's repos.
use std::path::PathBuf;

pub fn dir() -> Option<PathBuf> {
    std::env::var_os("SHEPHERD_MOCK").map(PathBuf::from)
}

pub fn snapshot() -> Option<(crate::model::Snapshot, Vec<(String, u64)>)> {
    let text = std::fs::read_to_string(dir()?.join("snapshot.json")).ok()?;
    let home = crate::settings::home();
    let v: serde_json::Value = serde_json::from_str(&text.replace("$HOME", &home.to_string_lossy())).ok()?;
    let snapshot = crate::model::Snapshot::from_result(&v).ok()?;
    let waited = v["waited"].as_object().into_iter().flatten().filter_map(|(k, s)| Some((k.clone(), s.as_u64()?))).collect();
    Some((snapshot, waited))
}

/// What the pane shows, as terminal output.
pub fn pane_text(pane_id: &str) -> Option<Vec<u8>> {
    let text = std::fs::read_to_string(dir()?.join("panes").join(format!("{pane_id}.txt"))).ok()?;
    Some(text.trim_end_matches('\n').replace("\\e", "\x1b").replace('\n', "\r\n").into_bytes())
}

/// The fixture's stand-in for a CLI, when it has one.
pub fn tool(name: &str) -> Option<String> {
    let path = dir()?.join("bin").join(name);
    path.is_file().then(|| path.to_string_lossy().into_owned())
}

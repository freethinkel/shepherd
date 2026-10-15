//! herdr's saved SSH machines (`herdr machine`). Shepherd reaches one through two local sockets: each connection to
//! them runs herdr's own stdio bridge on the host over SSH (`remote-api-bridge`, `remote-client-bridge`), so the rest
//! of Shepherd talks to it as to a local session.
use std::collections::HashMap;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::Mutex;

/// A saved machine, as `herdr machine list --json` lists it.
#[derive(Debug, Clone, PartialEq, serde::Deserialize)]
pub struct Machine {
    pub id: String,
    pub label: String,
    pub target: String,
    pub session: String,
    #[serde(default = "enabled")]
    pub enabled: bool,
}

fn enabled() -> bool {
    true
}

/// herdr's saved machines.
pub fn machines() -> anyhow::Result<Vec<Machine>> {
    Ok(serde_json::from_slice(&herdr(&["machine", "list", "--json"])?)?)
}

/// Saves `target` as a machine running the session `session`: herdr checks the host's herdr and starts that
/// session's server first. Setup that needs approval (installing or replacing herdr) only runs in a terminal.
pub fn add_machine(target: &str, session: &str) -> anyhow::Result<Machine> {
    herdr(&["machine", "add", target, "--remote-session", session]).map_err(|e| {
        anyhow::anyhow!("{e:#}\nIf the host needs herdr set up, run `herdr machine add {target} --remote-session {session}` in a terminal.")
    })?;
    machines()?
        .into_iter()
        .find(|m| m.target == target && m.session == session)
        .ok_or_else(|| anyhow::anyhow!("herdr saved no machine for {target}"))
}

/// Renames the machine's label.
pub fn rename_machine(id: &str, label: &str) -> anyhow::Result<()> {
    herdr(&["machine", "rename", id, "--label", label]).map(|_| ())
}

/// Forgets the machine; its sessions keep running on the host.
pub fn remove_machine(id: &str) -> anyhow::Result<()> {
    herdr(&["machine", "remove", id]).map(|_| ())
}

fn herdr(args: &[&str]) -> anyhow::Result<Vec<u8>> {
    let out = Command::new(super::session::binary()).args(args).stdin(Stdio::null()).output()?;
    anyhow::ensure!(
        out.status.success(),
        "herdr {}: {}",
        args[..2.min(args.len())].join(" "),
        String::from_utf8_lossy(&out.stderr).trim()
    );
    Ok(out.stdout)
}

/// The SSH target of the machine shown; `None` while a local session is.
static SHOWN: Mutex<Option<String>> = Mutex::new(None);

/// Marks a located directory as a machine's: `MARK target MARK path`.
const MARK: char = '\u{1}';

/// The machine shown from now on, by its SSH target; `None` for a local session.
pub fn show_machine(target: Option<String>) {
    *SHOWN.lock().unwrap() = target;
}

/// A directory of the place shown, as the rest of Shepherd keeps it: a machine's path is that machine's, and git
/// reaches it over SSH.
pub fn locate(path: &str) -> String {
    match SHOWN.lock().unwrap().as_deref() {
        Some(target) => format!("{MARK}{target}{MARK}{path}"),
        None => path.to_string(),
    }
}

/// A located directory's machine, by its SSH target (`None` here), and its path there.
pub fn split(dir: &str) -> (Option<&str>, &str) {
    dir.strip_prefix(MARK).and_then(|rest| rest.split_once(MARK)).map_or((None, dir), |(target, path)| (Some(target), path))
}

/// `git -C <path> <args>` for a located directory: here, or on its machine over the shared SSH connection.
pub fn git(dir: &str, args: &[&str]) -> std::io::Result<std::process::Output> {
    match split(dir) {
        (None, path) => Command::new("git").arg("-C").arg(path).args(args).stdin(Stdio::null()).output(),
        (Some(target), path) => {
            let words: Vec<String> = ["git", "-C", path].iter().chain(args).map(|w| quote(w)).collect();
            ssh(target, &words.join(" "))
        }
    }
}

/// `script` run by `sh` on the machine at `target` (its login shell may be fish), over the shared SSH connection.
pub fn ssh(target: &str, script: &str) -> std::io::Result<std::process::Output> {
    Command::new("ssh").args(ssh_options()).arg("--").arg(target).arg(format!("sh -c {}", quote(script))).stdin(Stdio::null()).output()
}

/// `word` for the remote shell, as one argument.
pub fn quote(word: &str) -> String {
    format!("'{}'", word.replace('\'', "'\\''"))
}

/// The relays running, by machine id, to their API socket.
static RELAYS: Mutex<Option<HashMap<String, PathBuf>>> = Mutex::new(None);

// ponytail: a relay lives until Shepherd quits; stop it on a switch away if idle SSH masters ever matter.
/// The local API socket that reaches `machine`, starting its relay on first use. Its client socket is the
/// `herdr-client.sock` next to it, where Shepherd looks for one.
pub fn relay(machine: &Machine, rt: &tokio::runtime::Handle) -> anyhow::Result<PathBuf> {
    let mut relays = RELAYS.lock().unwrap();
    let relays = relays.get_or_insert_with(HashMap::new);
    if let Some(api) = relays.get(&machine.id) {
        return Ok(api.clone());
    }
    let dir = cache_dir().join("r").join(short(&machine.id));
    std::fs::create_dir_all(&dir)?;
    let api = dir.join("herdr.sock");
    for (path, bridge) in [(api.clone(), "remote-api-bridge"), (dir.join("herdr-client.sock"), "remote-client-bridge")] {
        let _ = std::fs::remove_file(&path); // left by an earlier Shepherd
        let listener = {
            let _guard = rt.enter();
            tokio::net::UnixListener::bind(&path)?
        };
        let (target, command) = (machine.target.clone(), bridge_command(&machine.session, bridge));
        rt.spawn(async move {
            while let Ok((stream, _)) = listener.accept().await {
                let (target, command) = (target.clone(), command.clone());
                tokio::spawn(async move { pipe(stream, &target, &command).await });
            }
        });
    }
    relays.insert(machine.id.clone(), api.clone());
    Ok(api)
}

/// One local connection, carried over SSH to the bridge.
async fn pipe(stream: tokio::net::UnixStream, target: &str, command: &str) {
    let Ok(mut ssh) = tokio::process::Command::new("ssh")
        .args(ssh_options())
        .arg("--")
        .arg(target)
        .arg(command)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .kill_on_drop(true)
        .spawn()
    else {
        return;
    };
    let (Some(mut to_ssh), Some(mut from_ssh)) = (ssh.stdin.take(), ssh.stdout.take()) else { return };
    let (mut read, mut write) = stream.into_split();
    let up = async {
        let _ = tokio::io::copy(&mut read, &mut to_ssh).await;
        drop(to_ssh); // the bridge sees EOF and ends
    };
    let down = async {
        let _ = tokio::io::copy(&mut from_ssh, &mut write).await;
    };
    tokio::select! {
        _ = async { tokio::join!(up, down) } => {}
        _ = ssh.wait() => {}
    }
}

/// Never prompts (there is no terminal); one shared SSH connection per host, as herdr's own machines keep.
fn ssh_options() -> Vec<String> {
    let control = cache_dir().join("cm-%C");
    [
        "-C",
        "-o", "BatchMode=yes",
        "-o", "ConnectTimeout=10",
        "-o", "ServerAliveInterval=15",
        "-o", "ServerAliveCountMax=4",
        "-o", "ControlMaster=auto",
        "-o", "ControlPersist=600",
    ]
    .into_iter()
    .map(String::from)
    .chain(["-o".into(), format!("ControlPath={}", control.display())])
    .collect()
}

/// The shell command that runs `bridge` for `session` on the host, with herdr where its installers put it: a
/// non-interactive SSH shell has a bare PATH.
fn bridge_command(session: &str, bridge: &str) -> String {
    // the session's name is herdr's ASCII letters, digits, '.', '_' and '-', so it needs no quoting
    format!(
        "for h in herdr \"$HOME/.local/bin/herdr\" \"$HOME/.cargo/bin/herdr\" /opt/homebrew/bin/herdr /usr/local/bin/herdr; do \
         command -v \"$h\" >/dev/null 2>&1 && exec \"$h\" --session {session} {bridge}; done; \
         echo 'herdr is not installed on this host' >&2; exit 127"
    )
}

/// Under Shepherd's caches: short enough for a socket path (104 bytes on macOS).
fn cache_dir() -> PathBuf {
    crate::platform::cache_dir()
}

/// The first 12 characters of an id, for a short directory name.
fn short(id: &str) -> String {
    id.chars().filter(|c| c.is_ascii_alphanumeric()).take(12).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn machines_from_herdrs_list() {
        let json = br#"[{"id":"m-1a2b","label":"workbox/agents","target":"you@workbox","session":"agents","enabled":true,"selected":false}]"#;
        let list: Vec<Machine> = serde_json::from_slice(json).unwrap();
        assert_eq!(list[0], Machine { id: "m-1a2b".into(), label: "workbox/agents".into(), target: "you@workbox".into(), session: "agents".into(), enabled: true });
    }

    #[test]
    fn the_bridge_runs_for_the_session() {
        let command = bridge_command("agents", "remote-client-bridge");
        assert!(command.contains("--session agents remote-client-bridge"));
        assert!(command.contains("\"$HOME/.local/bin/herdr\""));
    }

    #[test]
    fn relay_paths_fit_a_socket() {
        let api = cache_dir().join("r").join(short("0123456789abcdef-0123")).join("herdr-client.sock");
        assert!(api.as_os_str().len() < 104, "{}", api.display());
    }
}

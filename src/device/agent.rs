//! Agents in herdr's panes drive the device open in their workspace's Device tab: `shepherd device …`, which is
//! `shepherd-device …` on the PATH once the user allows it (the setup's Agent access step). A simulator through
//! simdriver's agent layer (`simdriver_cli`: `agent snapshot` with element refs, `agent tap e4`, `agent steps`…), an
//! emulator or a phone through adb pinned to it. The window writes which device each workspace shows (`publish`), the
//! agent's pane names its workspace (`HERDR_WORKSPACE_ID`), and each command marks its device so the tab shows an
//! agent at it (`acted`).
use super::{Device, Platform};
use std::collections::BTreeMap;
use std::path::PathBuf;
use std::time::{Duration, SystemTime};

/// Each workspace's device, as the window last wrote it.
fn open_file() -> PathBuf {
    crate::settings::cache_dir().join("agent-devices.json")
}

/// A file per device an agent acted on, holding its pane.
fn mark_file(id: &str) -> PathBuf {
    let name: String = id.chars().map(|c| if c.is_ascii_alphanumeric() || c == '-' || c == '.' { c } else { '_' }).collect();
    crate::settings::cache_dir().join("agent-marks").join(name)
}

/// The shell script on the PATH that runs `shepherd device`.
pub fn shim() -> PathBuf {
    std::env::home_dir().unwrap_or_default().join(".local/bin/shepherd-device")
}

/// Puts `shepherd-device` on the PATH, pointing at this binary, or takes it off.
pub fn set_access(on: bool) -> anyhow::Result<()> {
    let shim = shim();
    if !on {
        return match std::fs::remove_file(&shim) {
            Err(e) if e.kind() != std::io::ErrorKind::NotFound => Err(e.into()),
            _ => Ok(()),
        };
    }
    // ponytail: a shell script, so Windows gets none; agents there run `shepherd.exe device`
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let exe = std::env::current_exe()?.to_string_lossy().replace('\'', r"'\''");
        std::fs::create_dir_all(shim.parent().unwrap_or(&shim))?;
        std::fs::write(&shim, format!("#!/bin/sh\n# Shepherd's: lets agents drive the device open in their workspace (Device tab → Agent access).\nexec '{exe}' device \"$@\"\n"))?;
        std::fs::set_permissions(&shim, std::fs::Permissions::from_mode(0o755))?;
    }
    Ok(())
}

/// `{workspace: {"platform": "ios", "id": …, "name": …}}`.
fn open_json(open: &[(String, &Device)]) -> String {
    let map: BTreeMap<&str, serde_json::Value> =
        open.iter().map(|(ws, d)| (ws.as_str(), serde_json::json!({"platform": d.platform.tag(), "id": d.id, "name": d.name}))).collect();
    serde_json::to_string(&map).unwrap_or_default()
}

/// The window's side: writes each workspace's device, when that changed since `last`.
pub fn publish(open: &[(String, &Device)], last: &mut String) {
    let json = open_json(open);
    if json == *last {
        return;
    }
    let file = open_file();
    let tmp = file.with_extension("tmp");
    let written = std::fs::create_dir_all(file.parent().unwrap_or(&file)).and_then(|_| std::fs::write(&tmp, &json)).and_then(|_| std::fs::rename(&tmp, &file));
    match written {
        Ok(()) => *last = json,
        Err(e) => eprintln!("shepherd: {}: {e}", file.display()),
    }
}

/// The device open in the calling pane's workspace: its platform, id and name.
fn workspace() -> Option<(String, String, String)> {
    let ws = std::env::var("HERDR_WORKSPACE_ID").ok()?;
    lookup(&std::fs::read_to_string(open_file()).ok()?, &ws)
}

/// Workspace `ws`'s device in what `open_json` wrote.
fn lookup(text: &str, ws: &str) -> Option<(String, String, String)> {
    let map: BTreeMap<String, serde_json::Value> = serde_json::from_str(text).ok()?;
    let d = map.get(ws)?;
    let s = |k: &str| d.get(k).and_then(|v| v.as_str()).map(str::to_string);
    Some((s("platform")?, s("id")?, s("name")?))
}

/// The workspace's device, when it's a `platform` one.
#[cfg_attr(not(target_os = "macos"), allow(dead_code))]
pub(super) fn workspace_device(platform: Platform) -> Option<String> {
    workspace().filter(|(p, ..)| p == platform.tag()).map(|(_, id, _)| id)
}

/// The agent is at device `id`: its tab says so for a while.
pub(super) fn acting(id: &str) {
    let file = mark_file(id);
    let pane = std::env::var("HERDR_PANE_ID").unwrap_or_default();
    let _ = std::fs::create_dir_all(file.parent().unwrap_or(&file)).and_then(|_| std::fs::write(&file, pane));
}

/// How long a command keeps its device marked.
const MARKED: Duration = Duration::from_secs(10);

/// The window's side: whether an agent acted on device `id` lately, and its pane (empty when it ran outside herdr).
pub fn acted(id: &str) -> Option<String> {
    let file = mark_file(id);
    let when = std::fs::metadata(&file).and_then(|m| m.modified()).ok()?;
    (SystemTime::now().duration_since(when).unwrap_or_default() < MARKED).then(|| std::fs::read_to_string(&file).unwrap_or_default())
}

const HELP: &str = "Drive the device open in this workspace's Device tab in Shepherd.

Usage: shepherd-device <command> [arguments]   (the same as `shepherd device …`)

  which                    The workspace's device, and how to drive it.

An iOS Simulator (simdriver's agent layer):
  agent snapshot           The screen's elements, each with a ref (e4); --image <path> saves it annotated.
  agent tap <target>       A ref, a label on screen (\"Log in\"), a grid cell (C4) or a point (@201,816).
  agent steps <json>       Several actions in one go.
  …                        `shepherd-device --help` on a workspace with a simulator lists every command.

An Android emulator or phone:
  screenshot [<path>]      The screen as a PNG; prints the path.
  adb <arguments>          adb pinned to the device: adb shell input tap 540 1200, adb shell uiautomator dump /dev/tty…
";

/// `shepherd device …`: what the agent asked, on its workspace's device.
pub fn cli(args: &[String]) -> i32 {
    let (settings, _) = crate::settings::Settings::load(&crate::settings::Settings::path());
    if !settings.devices_agent_access {
        eprintln!("shepherd-device: agents may not control devices: the user can allow it in Shepherd's Device tab (Agent access)");
        return 1;
    }
    let open = workspace();
    let android = open.as_ref().filter(|(p, ..)| p == Platform::Android.tag()).map(|(_, id, _)| id.clone());
    match (args.first().map(String::as_str), android) {
        (Some("which"), _) => which(open),
        (Some("adb"), Some(id)) => adb(&id, &args[1..]),
        (Some("screenshot"), Some(id)) => android_screenshot(&id, args.get(1)),
        (None | Some("-h" | "--help" | "help"), Some(_)) => {
            print!("{HELP}");
            0
        }
        (Some(other), Some(_)) => {
            eprintln!("shepherd-device: {other}: an Android device takes `screenshot` and `adb …`\n\n{HELP}");
            2
        }
        #[cfg(target_os = "macos")]
        (_, None) => super::simdriver_cli::cli(args),
        #[cfg(not(target_os = "macos"))]
        (_, None) => {
            eprintln!("shepherd-device: no Android device is open in this workspace's Device tab\n\n{HELP}");
            1
        }
    }
}

fn which(open: Option<(String, String, String)>) -> i32 {
    let Some((platform, id, name)) = open else {
        eprintln!("shepherd-device: no device is open in this workspace's Device tab; ask the user to open one (right panel → Device)");
        return 1;
    };
    let how = match platform.as_str() {
        "android" => "drive it with: shepherd-device screenshot, shepherd-device adb shell input tap X Y",
        _ => "drive it with: shepherd-device agent snapshot, then shepherd-device agent tap e4",
    };
    println!("{name} ({platform}, {id})\n{how}");
    0
}

fn adb(id: &str, args: &[String]) -> i32 {
    let run = || -> anyhow::Result<i32> {
        let serial = super::android::serial(id)?;
        acting(id);
        let status = crate::process::command(super::android::adb_path()?).arg("-s").arg(serial).args(args).status()?;
        Ok(status.code().unwrap_or(1))
    };
    run().unwrap_or_else(|e| {
        eprintln!("shepherd-device: {e:#}");
        1
    })
}

fn android_screenshot(id: &str, to: Option<&String>) -> i32 {
    let stamp = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).map_or(0, |d| d.as_millis());
    let to = to.map(PathBuf::from).unwrap_or_else(|| crate::settings::cache_dir().join("shots").join(format!("{}-{stamp}.png", mark_file(id).file_name().unwrap_or_default().to_string_lossy())));
    acting(id);
    let saved = to.parent().map_or(Ok(()), std::fs::create_dir_all).map_err(anyhow::Error::from).and_then(|_| super::android::screenshot(id, &to));
    match saved {
        Ok(()) => {
            println!("{}", to.display());
            0
        }
        Err(e) => {
            eprintln!("shepherd-device: {e:#}");
            1
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn workspaces_devices_as_written() {
        let d = Device { platform: Platform::Ios, id: "ABC-1".into(), name: "iPhone 17".into(), version: "iOS 27.0".into(), running: true, duo: false, usb: false };
        let json = open_json(&[("w1".into(), &d)]);
        assert_eq!(lookup(&json, "w1"), Some(("ios".into(), "ABC-1".into(), "iPhone 17".into())));
        assert_eq!(lookup(&json, "w2"), None);
        assert!(mark_file("emulator-5554:x/y").ends_with("agent-marks/emulator-5554_x_y"));
    }
}

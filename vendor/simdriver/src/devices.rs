use crate::Error;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::io::{Read, Write};
use std::os::fd::AsRawFd;
use std::path::{Component, Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

const RUNTIME_PREFIX: &str = "com.apple.CoreSimulator.SimRuntime.";
const BOOT_TIMEOUT: Duration = Duration::from_secs(60);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Device {
    pub udid: String,
    pub name: String,
    pub state: String,
    pub device_type: String,
    pub runtime: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct App {
    pub bundle_id: String,
    pub name: String,
    pub app_type: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DeviceSelector {
    Udid(String),
    FirstBootedOrIphone,
}

impl DeviceSelector {
    pub fn resolve(&self, devices: &[Device]) -> Result<Device, Error> {
        let found = match self {
            DeviceSelector::Udid(u) => devices.iter().find(|d| &d.udid == u),
            DeviceSelector::FirstBootedOrIphone => devices
                .iter()
                .find(|d| d.state == "Booted")
                .or_else(|| devices.iter().find(|d| d.device_type.contains("iPhone"))),
        };
        found.cloned().ok_or(Error::NoDevice)
    }
}

#[derive(Deserialize)]
struct Raw {
    devices: std::collections::BTreeMap<String, Vec<RawDevice>>,
}

#[derive(Deserialize)]
struct RawDevice {
    udid: String,
    name: String,
    state: String,
    #[serde(rename = "deviceTypeIdentifier", default)]
    device_type: String,
    #[serde(rename = "isAvailable", default = "yes")]
    is_available: bool,
}

fn yes() -> bool {
    true
}

pub(crate) fn parse_devices(json: &str) -> Result<Vec<Device>, Error> {
    let raw: Raw = serde_json::from_str(json)
        .map_err(|e| Error::Simctl(format!("bad simctl device json: {e}")))?;
    let mut out = Vec::new();
    for (runtime, list) in raw.devices {
        let runtime = runtime
            .strip_prefix(RUNTIME_PREFIX)
            .unwrap_or(&runtime)
            .to_string();
        for d in list.into_iter().filter(|d| d.is_available) {
            out.push(Device {
                udid: d.udid,
                name: d.name,
                state: d.state,
                device_type: d.device_type,
                runtime: runtime.clone(),
            });
        }
    }
    Ok(out)
}

/// Run `xcrun simctl <args>`; returns (success, stdout, stderr). Kills the child after
/// `timeout`.
fn run_simctl(args: &[&str], timeout: Duration) -> Result<(bool, String, String), Error> {
    run("xcrun", &[&["simctl"], args].concat(), None, timeout)
}

/// [`run_simctl`] with `input` written to the child's stdin.
fn run_simctl_input(
    args: &[&str],
    input: &[u8],
    timeout: Duration,
) -> Result<(bool, String, String), Error> {
    run("xcrun", &[&["simctl"], args].concat(), Some(input), timeout)
}

/// Run `program`, feeding it `input` (if any) and draining both pipes on threads so
/// neither side can block the other; kills the child after `timeout`.
fn run(
    program: &str,
    args: &[&str],
    input: Option<&[u8]>,
    timeout: Duration,
) -> Result<(bool, String, String), Error> {
    let mut child = Command::new(program)
        .args(args)
        .stdin(if input.is_some() {
            Stdio::piped()
        } else {
            Stdio::null()
        })
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;
    let drain = |mut pipe: Box<dyn Read + Send>| {
        std::thread::spawn(move || {
            let mut s = String::new();
            let _ = pipe.read_to_string(&mut s);
            s
        })
    };
    let out_t = drain(Box::new(child.stdout.take().unwrap()));
    let err_t = drain(Box::new(child.stderr.take().unwrap()));
    if let (Some(input), Some(mut stdin)) = (input, child.stdin.take()) {
        // A child that exits before reading everything must fail the write with EPIPE,
        // not kill a host that does not ignore SIGPIPE (C/Swift hosts don't).
        unsafe {
            libc::fcntl(stdin.as_raw_fd(), libc::F_SETNOSIGPIPE, 1);
        }
        let input = input.to_vec();
        // On a thread: a child that never reads must not stall us past the timeout.
        std::thread::spawn(move || {
            let _ = stdin.write_all(&input);
        });
    }
    let deadline = Instant::now() + timeout;
    let status = loop {
        if let Some(st) = child.try_wait()? {
            break st;
        }
        if Instant::now() >= deadline {
            let _ = child.kill();
            let _ = child.wait();
            return Err(Error::Simctl(format!(
                "{program} {} timed out after {}s",
                args.join(" "),
                timeout.as_secs()
            )));
        }
        std::thread::sleep(Duration::from_millis(50));
    };
    let out = out_t.join().unwrap_or_default();
    let err = err_t.join().unwrap_or_default();
    Ok((status.success(), out, err))
}

pub fn list_devices() -> Result<Vec<Device>, Error> {
    let (ok, out, err) = run_simctl(&["list", "devices", "-j"], Duration::from_secs(30))?;
    if !ok {
        return Err(Error::Simctl(err.trim().to_string()));
    }
    parse_devices(&out)
}

/// List installed simulator apps (including system apps) by bundle identifier.
pub fn list_apps(udid: &str) -> Result<Vec<App>, Error> {
    let (ok, plist, err) = run_simctl(&["listapps", udid], Duration::from_secs(30))?;
    if !ok {
        return Err(Error::Simctl(err.trim().to_string()));
    }
    parse_apps(&plist)
}

fn parse_apps(plist: &str) -> Result<Vec<App>, Error> {
    let json = plist_json(plist)?;
    let entries: std::collections::BTreeMap<String, Value> = serde_json::from_str(&json)
        .map_err(|e| Error::Simctl(format!("bad simctl app list: {e}")))?;
    let mut apps = entries
        .into_iter()
        .map(|(bundle_id, value)| {
            let name = value
                .get("CFBundleDisplayName")
                .and_then(Value::as_str)
                .or_else(|| value.get("CFBundleName").and_then(Value::as_str))
                .unwrap_or(&bundle_id)
                .to_string();
            let app_type = value
                .get("ApplicationType")
                .and_then(Value::as_str)
                .unwrap_or("Unknown")
                .to_string();
            App {
                bundle_id,
                name,
                app_type,
            }
        })
        .collect::<Vec<_>>();
    apps.sort_by(|a, b| {
        a.name
            .to_lowercase()
            .cmp(&b.name.to_lowercase())
            .then_with(|| a.bundle_id.cmp(&b.bundle_id))
    });
    Ok(apps)
}

fn plist_json(plist: &str) -> Result<String, Error> {
    let (ok, json, err) = run(
        "plutil",
        &["-convert", "json", "-o", "-", "-"],
        Some(plist.as_bytes()),
        Duration::from_secs(30),
    )?;
    if ok {
        Ok(json)
    } else {
        Err(Error::Simctl(format!(
            "plutil could not parse simctl app list: {}",
            err.trim()
        )))
    }
}

/// Explicitly terminate an app and clear the contents of its simulator data container.
/// This is destructive to the app's saved state and is never called implicitly.
pub fn clear_app_data(udid: &str, bundle_id: &str) -> Result<(), Error> {
    if udid.len() != 36 || !udid.bytes().all(|b| b.is_ascii_hexdigit() || b == b'-') {
        return Err(Error::Simctl("invalid simulator UDID".into()));
    }
    if bundle_id.is_empty()
        || bundle_id.contains('/')
        || bundle_id.contains('\\')
        || bundle_id.contains('\0')
    {
        return Err(Error::Simctl("invalid app bundle identifier".into()));
    }
    let (ok, path, err) = run_simctl(
        &["get_app_container", udid, bundle_id, "data"],
        Duration::from_secs(30),
    )?;
    if !ok {
        return Err(Error::Simctl(format!(
            "get app data container failed: {}",
            err.trim()
        )));
    }
    let container = PathBuf::from(path.trim()).canonicalize()?;
    if !is_sim_app_data_container(&container, udid) {
        return Err(Error::Simctl(format!(
            "refusing to clear unexpected data container: {}",
            container.display()
        )));
    }
    let (terminated, _, terminate_err) =
        run_simctl(&["terminate", udid, bundle_id], Duration::from_secs(30))?;
    // Not running is the usual case: nothing to terminate before erasing. simctl words it
    // "found nothing to terminate" (ESRCH) and, on some versions, "app is not running".
    if !terminated
        && !terminate_err.to_ascii_lowercase().contains("not running")
        && !terminate_err
            .to_ascii_lowercase()
            .contains("found nothing to terminate")
    {
        return Err(Error::Simctl(format!(
            "could not terminate app before clearing data: {}",
            terminate_err.trim()
        )));
    }
    for entry in std::fs::read_dir(&container)? {
        let entry = entry?;
        let path = entry.path();
        let kind = entry.file_type()?;
        if kind.is_dir() {
            std::fs::remove_dir_all(path)?;
        } else {
            std::fs::remove_file(path)?;
        }
    }
    Ok(())
}

fn is_sim_app_data_container(path: &Path, udid: &str) -> bool {
    let c = path
        .components()
        .filter_map(|c| match c {
            Component::Normal(s) => s.to_str(),
            _ => None,
        })
        .collect::<Vec<_>>();
    c.len() >= 7
        && c[c.len() - 7..c.len() - 1]
            == ["Devices", udid, "data", "Containers", "Data", "Application"]
        && c.last()
            .is_some_and(|s| s.len() == 36 && s.bytes().all(|b| b.is_ascii_hexdigit() || b == b'-'))
}

/// Boot headlessly and wait until ready. Already-booted is success.
pub fn boot(udid: &str) -> Result<(), Error> {
    let (ok, _, err) = run_simctl(&["bootstatus", udid, "-b"], BOOT_TIMEOUT)?;
    if ok || err.contains("already booted") || err.contains("current state: Booted") {
        return Ok(());
    }
    Err(Error::Simctl(format!(
        "bootstatus failed: {}",
        if err.trim().is_empty() {
            "(no stderr)"
        } else {
            err.trim()
        }
    )))
}

/// Shut down. Already-shutdown is success.
pub fn shutdown(udid: &str) -> Result<(), Error> {
    let (ok, _, err) = run_simctl(&["shutdown", udid], Duration::from_secs(30))?;
    if ok || err.contains("current state: Shutdown") {
        return Ok(());
    }
    Err(Error::Simctl(err.trim().to_string()))
}

/// Copy UTF-8 text to a simulator's pasteboard. An empty string clears it.
pub fn clipboard_write(udid: &str, text: &str) -> Result<(), Error> {
    let (ok, _, err) =
        run_simctl_input(&["pbcopy", udid], text.as_bytes(), Duration::from_secs(10))?;
    if ok {
        return Ok(());
    }
    // Some CoreSimulator runtimes apply the pasteboard write but then time out while
    // waiting for the host-side clipboard notification. Confirm the observable state
    // before treating that runtime response as a failure.
    match clipboard_read(udid) {
        Ok(current) if current == text => Ok(()),
        _ => Err(Error::Simctl(format!("pbcopy failed: {}", err.trim()))),
    }
}

/// Read UTF-8 text from a simulator's pasteboard.
pub fn clipboard_read(udid: &str) -> Result<String, Error> {
    let (ok, out, err) = run_simctl(&["pbpaste", udid], Duration::from_secs(10))?;
    if ok {
        Ok(out)
    } else {
        Err(Error::Simctl(format!("pbpaste failed: {}", err.trim())))
    }
}

fn simctl_ok(args: &[&str]) -> Result<(), Error> {
    let (ok, _, err) = run_simctl(args, Duration::from_secs(30))?;
    if ok {
        Ok(())
    } else {
        Err(Error::Simctl(err.trim().to_string()))
    }
}

pub fn open_url(udid: &str, url: &str) -> Result<(), Error> {
    simctl_ok(&["openurl", udid, url])
}

pub fn launch(udid: &str, bundle_id: &str) -> Result<(), Error> {
    simctl_ok(&["launch", udid, bundle_id])
}

pub fn terminate(udid: &str, bundle_id: &str) -> Result<(), Error> {
    simctl_ok(&["terminate", udid, bundle_id])
}

pub fn set_location(udid: &str, lat: f64, lon: f64) -> Result<(), Error> {
    if !lat.is_finite()
        || !(-90.0..=90.0).contains(&lat)
        || !lon.is_finite()
        || !(-180.0..=180.0).contains(&lon)
    {
        return Err(Error::Simctl("latitude/longitude out of range".into()));
    }
    let coordinates = format!("{lat},{lon}");
    simctl_ok(&["location", udid, "set", &coordinates])
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Appearance {
    Light,
    Dark,
}

pub fn set_appearance(udid: &str, appearance: Appearance) -> Result<(), Error> {
    simctl_ok(&[
        "ui",
        udid,
        "appearance",
        match appearance {
            Appearance::Light => "light",
            Appearance::Dark => "dark",
        },
    ])
}

pub fn grant_permission(udid: &str, service: &str, bundle_id: &str) -> Result<(), Error> {
    simctl_ok(&["privacy", udid, "grant", service, bundle_id])
}

pub fn revoke_permission(udid: &str, service: &str, bundle_id: &str) -> Result<(), Error> {
    simctl_ok(&["privacy", udid, "revoke", service, bundle_id])
}

pub fn reset_permission(udid: &str, service: &str, bundle_id: &str) -> Result<(), Error> {
    simctl_ok(&["privacy", udid, "reset", service, bundle_id])
}

pub fn set_content_size(udid: &str, size: &str) -> Result<(), Error> {
    simctl_ok(&["ui", udid, "content_size", size])
}

pub fn status_bar_override(udid: &str) -> Result<(), Error> {
    simctl_ok(&[
        "status_bar",
        udid,
        "override",
        "--time",
        "9:41",
        "--batteryLevel",
        "100",
        "--batteryState",
        "charged",
    ])
}

pub fn status_bar_clear(udid: &str) -> Result<(), Error> {
    simctl_ok(&["status_bar", udid, "clear"])
}

pub fn push_notification(udid: &str, bundle_id: &str, payload_path: &str) -> Result<(), Error> {
    simctl_ok(&["push", udid, bundle_id, payload_path])
}

/// Runs `sdaxsettings` inside the simulator: `status` (a JSON object of every key), `get <key>`, `set <key> <value>`,
/// for the accessibility settings `simctl ui` doesn't cover (reduce-motion, show-borders, reduce-transparency,
/// voiceover: on|off; color-filter: none|grayscale|red-green|green-red|blue-yellow; liquid-glass: clear|tinted).
/// What it printed.
pub fn ax_settings(udid: &str, args: &[&str]) -> Result<String, Error> {
    let helper = crate::helper::helper_dir(None).join("sdaxsettings");
    let helper = helper.to_string_lossy();
    let (ok, out, err) = run_simctl(&[&["spawn", udid, &helper], args].concat(), Duration::from_secs(30))?;
    if ok {
        Ok(out)
    } else {
        Err(Error::Simctl(format!("sdaxsettings failed: {}", err.trim())))
    }
}

pub fn reset_keychain(udid: &str) -> Result<(), Error> {
    simctl_ok(&["keychain", udid, "reset"])
}

#[cfg(test)]
mod tests {
    use super::*;

    const FIXTURE: &str = include_str!("../tests/fixtures/simctl_devices.json");

    fn dev(udid: &str, state: &str, ty: &str) -> Device {
        Device {
            udid: udid.into(),
            name: udid.into(),
            state: state.into(),
            device_type: ty.into(),
            runtime: "iOS-27-0".into(),
        }
    }

    #[test]
    fn parses_fixture() {
        let ds = parse_devices(FIXTURE).unwrap();
        assert_eq!(ds.len(), 3);
        let d = ds
            .iter()
            .find(|d| d.udid == "8BC981D2-7C86-4EFC-944C-AC33FD0262FC")
            .unwrap();
        assert_eq!(d.name, "iPhone 18 Pro");
        assert_eq!(d.state, "Booted");
        assert_eq!(d.runtime, "iOS-27-0");
        assert_eq!(
            d.device_type,
            "com.apple.CoreSimulator.SimDeviceType.iPhone-18-Pro"
        );
        assert!(ds.iter().any(|d| d.runtime == "iOS-27-1"));
    }

    #[test]
    fn parse_rejects_garbage() {
        assert!(matches!(parse_devices("nope"), Err(Error::Simctl(_))));
    }

    #[test]
    fn parses_simctl_app_plist_into_named_apps() {
        let plist = r#"{
          "com.example.zebra" = { ApplicationType = User; CFBundleName = Zebra; };
          "com.example.alpha" = { ApplicationType = User; CFBundleDisplayName = Alpha; CFBundleName = Fallback; };
        }"#;
        let apps = parse_apps(plist).unwrap();
        assert_eq!(apps[0].name, "Alpha");
        assert_eq!(apps[1].name, "Zebra");
    }

    #[test]
    fn app_data_clear_accepts_only_simulator_application_containers() {
        let udid = "8BC981D2-7C86-4EFC-944C-AC33FD0262FC";
        let good = Path::new(
            "/Users/test/Library/Developer/CoreSimulator/Devices/8BC981D2-7C86-4EFC-944C-AC33FD0262FC/data/Containers/Data/Application/B5D07B81-A27A-4A8C-84D6-031FFB375944",
        );
        let wrong_sim = good
            .to_string_lossy()
            .replace(udid, "00000000-0000-0000-0000-000000000000");
        let too_high = Path::new(
            "/Users/test/Library/Developer/CoreSimulator/Devices/8BC981D2-7C86-4EFC-944C-AC33FD0262FC/data/Containers/Data/Application",
        );
        let nested = Path::new(
            "/Users/test/Library/Developer/CoreSimulator/Devices/8BC981D2-7C86-4EFC-944C-AC33FD0262FC/data/Containers/Data/Application/B5D07B81-A27A-4A8C-84D6-031FFB375944/extra/A5D07B81-A27A-4A8C-84D6-031FFB375944",
        );
        assert!(is_sim_app_data_container(good, udid));
        assert!(!is_sim_app_data_container(Path::new(&wrong_sim), udid));
        assert!(!is_sim_app_data_container(too_high, udid));
        assert!(!is_sim_app_data_container(nested, udid));
    }

    #[test]
    fn resolve_exact_udid() {
        let ds = vec![dev("A", "Shutdown", "iPhone-X"), dev("B", "Booted", "iPad")];
        let d = DeviceSelector::Udid("A".into()).resolve(&ds).unwrap();
        assert_eq!(d.udid, "A");
    }

    #[test]
    fn resolve_unknown_udid() {
        let ds = vec![dev("A", "Shutdown", "iPhone-X")];
        assert!(matches!(
            DeviceSelector::Udid("Z".into()).resolve(&ds),
            Err(Error::NoDevice)
        ));
    }

    #[test]
    fn resolve_prefers_booted() {
        let ds = vec![dev("A", "Shutdown", "iPhone-X"), dev("B", "Booted", "iPad")];
        assert_eq!(
            DeviceSelector::FirstBootedOrIphone
                .resolve(&ds)
                .unwrap()
                .udid,
            "B"
        );
    }

    #[test]
    fn resolve_iphone_fallback() {
        let ds = vec![
            dev("A", "Shutdown", "iPad"),
            dev("B", "Shutdown", "iPhone-X"),
        ];
        assert_eq!(
            DeviceSelector::FirstBootedOrIphone
                .resolve(&ds)
                .unwrap()
                .udid,
            "B"
        );
    }

    #[test]
    fn resolve_empty() {
        assert!(matches!(
            DeviceSelector::FirstBootedOrIphone.resolve(&[]),
            Err(Error::NoDevice)
        ));
    }
}

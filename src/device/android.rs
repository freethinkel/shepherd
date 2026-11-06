//! Android emulators through their own gRPC (`emulator_controller.proto` in the SDK), as serve-emu's `grpc-screenshot`
//! source: raw RGB frames streamed on change, touches and keys sent back. A running emulator leaves a discovery file
//! with its gRPC port and token; booting is the SDK's `emulator`, headless, so the panel is its only window.
use super::tools::{self, Action, Decision, Settings, Toggle};
use super::{Button, Check, Device, Feed, Fold, Frame, Hinge, Platform, Touch, Wake};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tonic::codegen::http::uri::PathAndQuery;

/// How long a cold boot may take before the panel gives up on it.
const BOOT_TIMEOUT: Duration = Duration::from_secs(180);
const SERVICE: &str = "/android.emulation.control.EmulatorController/";

// The few messages used, by hand: the proto's field numbers, no build step.
#[derive(Clone, PartialEq, prost::Message)]
struct ImageFormat {
    /// 2: RGB888.
    #[prost(int32, tag = "1")]
    format: i32,
    /// The frame's: how the device is held, the image turned to match.
    #[prost(message, optional, tag = "2")]
    rotation: Option<Rotation>,
    #[prost(uint32, tag = "3")]
    width: u32,
    #[prost(uint32, tag = "4")]
    height: u32,
}

#[derive(Clone, PartialEq, prost::Message)]
struct Rotation {
    /// 0 portrait, 1 landscape (a quarter turn), 2 upside down, 3 the other landscape.
    #[prost(int32, tag = "1")]
    rotation: i32,
}

/// The quarter turns clockwise (`Frame::turn`) a frame of the emulator's `rotation` shows its screen turned by: its
/// landscape is a turn counterclockwise (the status bar on the left), as Android Studio's emulator view reads it.
fn turn(rotation: i32) -> u8 {
    match rotation {
        1 => 3,
        2 => 2,
        3 => 1,
        _ => 0,
    }
}

#[derive(Clone, PartialEq, prost::Message)]
struct Image {
    #[prost(message, optional, tag = "1")]
    format: Option<ImageFormat>,
    #[prost(bytes = "vec", tag = "4")]
    image: Vec<u8>,
}

#[derive(Clone, PartialEq, prost::Message)]
struct TouchEvent {
    #[prost(message, repeated, tag = "1")]
    touches: Vec<TouchPoint>,
}

#[derive(Clone, PartialEq, prost::Message)]
struct TouchPoint {
    #[prost(int32, tag = "1")]
    x: i32,
    #[prost(int32, tag = "2")]
    y: i32,
    #[prost(int32, tag = "3")]
    identifier: i32,
    /// 0 lifts the finger.
    #[prost(int32, tag = "4")]
    pressure: i32,
}

#[derive(Clone, PartialEq, prost::Message)]
struct KeyboardEvent {
    /// 2: keypress, down and up.
    #[prost(int32, tag = "2")]
    event_type: i32,
    /// A W3C key name: `GoHome`, `Enter`.
    #[prost(string, tag = "4")]
    key: String,
    #[prost(string, tag = "5")]
    text: String,
}

#[derive(Clone, PartialEq, prost::Message)]
struct Empty {}

/// The SDK: `ANDROID_HOME`, else where Android Studio puts it.
fn sdk() -> Option<PathBuf> {
    let home = std::env::var_os("HOME").or_else(|| std::env::var_os("USERPROFILE")).map(PathBuf::from).unwrap_or_default();
    let local = std::env::var_os("LOCALAPPDATA").map(|d| PathBuf::from(d).join("Android/Sdk"));
    let env = ["ANDROID_HOME", "ANDROID_SDK_ROOT"].into_iter().filter_map(std::env::var_os).map(PathBuf::from);
    env.chain([home.join("Library/Android/sdk"), home.join("Android/Sdk")]).chain(local).find(|p| p.is_dir())
}

fn tool(sdk: &Path, dir: &str, name: &str) -> PathBuf {
    sdk.join(dir).join(format!("{name}{}", std::env::consts::EXE_SUFFIX))
}

fn avd_home() -> PathBuf {
    if let Some(dir) = std::env::var_os("ANDROID_AVD_HOME") {
        return dir.into();
    }
    let home = std::env::var_os("HOME").or_else(|| std::env::var_os("USERPROFILE")).map(PathBuf::from).unwrap_or_default();
    std::env::var_os("ANDROID_USER_HOME").map(PathBuf::from).unwrap_or_else(|| home.join(".android")).join("avd")
}

/// `key=value` lines.
fn ini(text: &str) -> HashMap<String, String> {
    text.lines().filter_map(|l| l.split_once('=')).map(|(k, v)| (k.trim().to_string(), v.trim().to_string())).collect()
}

/// AVD `id`'s `config.ini`.
fn config(id: &str) -> HashMap<String, String> {
    let home = avd_home();
    let pointer = std::fs::read_to_string(home.join(format!("{id}.ini"))).map(|t| ini(&t)).unwrap_or_default();
    let dir = pointer.get("path").map(PathBuf::from).unwrap_or_else(|| home.join(format!("{id}.avd")));
    std::fs::read_to_string(dir.join("config.ini")).map(|t| ini(&t)).unwrap_or_default()
}

/// `system-images/android-36/google_apis/arm64-v8a/` → `API 36`.
fn api_level(sysdir: &str) -> Option<String> {
    sysdir.split('/').find_map(|p| p.strip_prefix("android-")).map(|n| format!("API {n}"))
}

/// The screen's native size in pixels, portrait.
fn lcd(config: &HashMap<String, String>) -> Option<(u32, u32)> {
    Some((config.get("hw.lcd.width")?.parse().ok()?, config.get("hw.lcd.height")?.parse().ok()?))
}

/// A running emulator, from its discovery file.
struct Running {
    serial: String,
    port: u16,
    token: String,
}

/// The running emulators by AVD id. A file left by a dead one is skipped: its port no longer answers.
fn running() -> HashMap<String, Running> {
    let home = std::env::var_os("HOME").map(PathBuf::from).unwrap_or_default();
    let mut dirs = vec![home.join("Library/Caches/TemporaryItems/avd/running"), std::env::temp_dir().join("avd/running"), avd_home().join("running")];
    dirs.extend(std::env::var_os("XDG_RUNTIME_DIR").map(|d| PathBuf::from(d).join("avd/running")));
    dirs.extend(std::env::var_os("LOCALAPPDATA").map(|d| PathBuf::from(d).join("Temp/avd/running")));
    let mut found = HashMap::new();
    for entry in dirs.iter().filter_map(|d| std::fs::read_dir(d).ok()).flatten().flatten() {
        let name = entry.file_name().to_string_lossy().to_string();
        if !(name.starts_with("pid_") && name.ends_with(".ini")) {
            continue;
        }
        let Ok(text) = std::fs::read_to_string(entry.path()) else { continue };
        let v = ini(&text);
        let (Some(id), Some(console), Some(port)) = (v.get("avd.id"), v.get("port.serial"), v.get("grpc.port").and_then(|p| p.parse::<u16>().ok())) else { continue };
        let alive = std::net::TcpStream::connect_timeout(&([127, 0, 0, 1], port).into(), Duration::from_millis(200)).is_ok();
        if alive {
            found.insert(id.clone(), Running { serial: format!("emulator-{console}"), port, token: v.get("grpc.token").cloned().unwrap_or_default() });
        }
    }
    found
}

pub(super) fn find() -> (Vec<Device>, Check) {
    let Some(sdk) = sdk() else {
        return (Vec::new(), Check { ready: false, message: "The Android SDK was not found. Install Android Studio, or set ANDROID_HOME.".into() });
    };
    let emulator = tool(&sdk, "emulator", "emulator");
    if !emulator.exists() || !tool(&sdk, "platform-tools", "adb").exists() {
        let message = "Install Android Emulator and SDK Platform-Tools in Android Studio's SDK Manager.".into();
        return (Vec::new(), Check { ready: false, message });
    }
    let listed = crate::process::command(&emulator).arg("-list-avds").output().map(|o| String::from_utf8_lossy(&o.stdout).to_string()).unwrap_or_default();
    let live = running();
    let devices: Vec<Device> = listed
        .lines()
        .map(str::trim)
        // the launcher also prints `INFO | …` lines
        .filter(|l| !l.is_empty() && !l.contains(' '))
        .map(|id| {
            let c = config(id);
            let name = c.get("avd.ini.displayname").cloned().unwrap_or_else(|| id.replace('_', " "));
            let version = c.get("image.sysdir.1").and_then(|s| api_level(s)).unwrap_or_default();
            Device { platform: Platform::Android, id: id.to_string(), name, version, running: live.contains_key(id), duo: false, usb: false }
        })
        .collect();
    #[cfg(target_os = "macos")]
    let devices = [devices, phones()].concat();
    let check = match devices.is_empty() {
        true => Check { ready: false, message: "The Android SDK is installed, but no virtual device exists. Create one in Android Studio → Device Manager.".into() },
        false => Check { ready: true, message: "The Android SDK and Emulator are available.".into() },
    };
    (devices, check)
}

pub(super) fn adb_path() -> anyhow::Result<PathBuf> {
    sdk().map(|sdk| tool(&sdk, "platform-tools", "adb")).ok_or_else(|| anyhow::anyhow!("The Android SDK was not found"))
}

fn adb(serial: &str, args: &[&str]) -> anyhow::Result<std::process::Output> {
    Ok(crate::process::command(adb_path()?).arg("-s").arg(serial).args(args).output()?)
}

/// What adb is connected to and can use: each one's serial and model.
fn attached() -> Vec<(String, String)> {
    let out = adb_path().and_then(|adb| Ok(crate::process::command(adb).args(["devices", "-l"]).output()?));
    out.map(|o| attached_lines(&String::from_utf8_lossy(&o.stdout))).unwrap_or_default()
}

/// `adb devices -l`'s lines: `R5CT… device usb:1-1 product:… model:SM_S911B …`; an unauthorized or offline one is
/// left out.
fn attached_lines(text: &str) -> Vec<(String, String)> {
    text.lines()
        .filter_map(|l| {
            let mut words = l.split_whitespace();
            let (serial, state) = (words.next()?, words.next()?);
            let model = words.find_map(|w| w.strip_prefix("model:")).unwrap_or(serial);
            (state == "device").then(|| (serial.to_string(), model.replace('_', " ")))
        })
        .collect()
}

/// Phones on USB (or adb over Wi-Fi): the attached devices that aren't emulators.
// ponytail: listed only where scrcpy's H.264 decodes (VideoToolbox, macOS); openh264 would bring Linux and Windows
#[cfg(target_os = "macos")]
fn phones() -> Vec<Device> {
    let phones = attached().into_iter().filter(|(serial, _)| !serial.starts_with("emulator-"));
    phones
        .map(|(serial, name)| {
            let sdk = adb(&serial, &["shell", "getprop", "ro.build.version.sdk"]).map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string());
            let version = sdk.ok().filter(|v| !v.is_empty()).map(|v| format!("API {v} · USB")).unwrap_or_else(|| "USB".into());
            Device { platform: Platform::Android, id: serial, name, version, running: true, duo: false, usb: true }
        })
        .collect()
}

/// A running emulator's adb serial, or a phone's own.
fn serial(id: &str) -> anyhow::Result<String> {
    if let Some(r) = running().remove(id) {
        return Ok(r.serial);
    }
    attached().into_iter().map(|(s, _)| s).find(|s| s == id).ok_or_else(|| anyhow::anyhow!("{id} is not running"))
}

pub(super) fn boot(id: &str) -> anyhow::Result<()> {
    if running().contains_key(id) {
        return Ok(());
    }
    let sdk = sdk().ok_or_else(|| anyhow::anyhow!("The Android SDK was not found"))?;
    let mut child = crate::process::command(tool(&sdk, "emulator", "emulator"))
        .args([&format!("@{id}"), "-no-window", "-no-audio", "-no-boot-anim"])
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::piped())
        .spawn()?;
    // its complaints, for when it quits; drained so it never blocks on a full pipe
    let stderr = child.stderr.take();
    let said = std::thread::spawn(move || {
        let mut text = String::new();
        if let Some(mut s) = stderr {
            let _ = std::io::Read::read_to_string(&mut s, &mut text);
        }
        text
    });
    let deadline = Instant::now() + BOOT_TIMEOUT;
    while Instant::now() < deadline {
        if let Some(status) = child.try_wait()?
            && !status.success()
        {
            let text = said.join().unwrap_or_default();
            let last = text.lines().rev().find(|l| !l.trim().is_empty()).unwrap_or("no output");
            anyhow::bail!("The emulator quit: {last}");
        }
        if let Some(r) = running().get(id)
            && adb(&r.serial, &["shell", "getprop", "sys.boot_completed"]).is_ok_and(|o| String::from_utf8_lossy(&o.stdout).trim() == "1")
        {
            // reaped once it powers off
            std::thread::spawn(move || child.wait());
            return Ok(());
        }
        std::thread::sleep(Duration::from_secs(1));
    }
    anyhow::bail!("The emulator did not finish starting in {} seconds", BOOT_TIMEOUT.as_secs())
}

pub(super) fn power_off(id: &str) -> anyhow::Result<()> {
    crate::process::answer(adb(&serial(id)?, &["emu", "kill"])?).map(drop)
}

pub(super) fn dark(id: &str) -> Option<bool> {
    let out = adb(&serial(id).ok()?, &["shell", "cmd", "uimode", "night"]).ok()?;
    let text = String::from_utf8_lossy(&out.stdout).to_string();
    text.contains("Night mode:").then(|| text.contains("yes"))
}

pub(super) fn set_dark(id: &str, dark: bool) -> anyhow::Result<()> {
    crate::process::answer(adb(&serial(id)?, &["shell", "cmd", "uimode", "night", if dark { "yes" } else { "no" }])?).map(drop)
}

/// `adb shell` with `args` quoted for the device's shell: what it printed.
fn shell(serial: &str, args: &[&str]) -> anyhow::Result<String> {
    let line: Vec<String> = args.iter().map(|a| tools::shell_word(a)).collect();
    crate::process::answer(adb(serial, &["shell", &line.join(" ")])?)
}

/// Read in one go, each answer after a line of its own.
const READ: &str = "cmd uimode night; echo @@; settings get system font_scale; echo @@; settings get global animator_duration_scale; \
    echo @@; settings get global wifi_on; echo @@; dumpsys window | grep -E 'mCurrentFocus|mFocusedApp'";

pub(super) fn read_tools(id: &str) -> Settings {
    let out = serial(id).and_then(|s| Ok(String::from_utf8_lossy(&adb(&s, &["shell", READ])?.stdout).to_string())).unwrap_or_default();
    read_answers(&out)
}

fn read_answers(out: &str) -> Settings {
    let mut part = out.split("@@").map(str::trim);
    let (night, font, animator, wifi, focus) = (part.next(), part.next(), part.next(), part.next(), part.next());
    let mut toggles = Vec::new();
    // `null` is a scale never set: the default, 1
    let animator = animator.and_then(|a| if a == "null" { Some(1.) } else { a.parse::<f64>().ok() });
    toggles.extend(animator.map(|a| (Toggle::ReduceMotion, a == 0.)));
    toggles.extend(wifi.filter(|w| *w == "0" || *w == "1").map(|w| (Toggle::Network, w == "1")));
    Settings {
        dark: night.filter(|n| n.contains("Night mode:")).map(|n| n.contains("yes")),
        text_size: font.and_then(|f| f.parse().ok()).map(tools::text_size_from_android),
        toggles,
        app: focus.and_then(focused_package),
        ..Default::default()
    }
}

/// The focused window's package: `mCurrentFocus=Window{8e3a1b u0 com.example/com.example.Main}`.
fn focused_package(dump: &str) -> Option<String> {
    let line = dump.lines().find(|l| l.contains("mCurrentFocus=")).or_else(|| dump.lines().find(|l| l.contains("mFocusedApp=")))?;
    let word = line.split_whitespace().find(|w| w.contains('/'))?;
    Some(word.split('/').next()?.to_string()).filter(|p| !p.is_empty())
}

/// t3code's permissions, as Android names them: a group grants each of its names the app declares.
fn android_permissions(permission: &str) -> &'static [&'static str] {
    match permission {
        "camera" => &["android.permission.CAMERA"],
        "microphone" => &["android.permission.RECORD_AUDIO"],
        "photos" => &["android.permission.READ_MEDIA_IMAGES", "android.permission.READ_EXTERNAL_STORAGE"],
        "contacts" => &["android.permission.READ_CONTACTS", "android.permission.WRITE_CONTACTS"],
        "calendar" => &["android.permission.READ_CALENDAR", "android.permission.WRITE_CALENDAR"],
        "location" => &["android.permission.ACCESS_FINE_LOCATION", "android.permission.ACCESS_COARSE_LOCATION"],
        "notifications" => &["android.permission.POST_NOTIFICATIONS"],
        "motion" => &["android.permission.ACTIVITY_RECOGNITION"],
        _ => &[],
    }
}

/// The emulator's gravity for an orientation, as a portrait-native device shows it: tilting its accelerometer turns
/// the display for real, where `user-rotation` turns only the windows on recent images.
fn gravity(orientation: &str) -> &'static str {
    match orientation {
        "landscape_left" => "9.81:0:0",
        "portrait_upside_down" => "0:-9.81:0",
        "landscape_right" => "-9.81:0:0",
        _ => "0:9.81:0",
    }
}

/// A landscape-native tablet's orientation is a quarter turn further along.
fn landscape_native(orientation: &str) -> &'static str {
    match orientation {
        "landscape_left" => "portrait",
        "portrait_upside_down" => "landscape_left",
        "landscape_right" => "portrait_upside_down",
        _ => "landscape_right",
    }
}

pub(super) fn act(id: &str, action: &Action) -> anyhow::Result<()> {
    let serial = serial(id)?;
    let sh = |args: &[&str]| shell(&serial, args).map(drop);
    match action {
        Action::Dark(dark) => set_dark(id, *dark)?,
        Action::TextSize(size) => sh(&["settings", "put", "system", "font_scale", tools::android_font_scale(*size)])?,
        Action::Toggle(Toggle::ReduceMotion, on) => {
            for key in ["animator_duration_scale", "transition_animation_scale", "window_animation_scale"] {
                sh(&["settings", "put", "global", key, if *on { "0" } else { "1" }])?;
            }
        }
        Action::Toggle(Toggle::Network, on) => {
            let state = if *on { "enable" } else { "disable" };
            sh(&["svc", "wifi", state])?;
            sh(&["svc", "data", state])?;
        }
        Action::Orientation(orientation) => {
            let size = display_size(&serial);
            let turned = if size.is_some_and(|(w, h)| w > h) { landscape_native(orientation) } else { orientation };
            sh(&["settings", "put", "system", "accelerometer_rotation", "1"])?;
            sh(&["cmd", "window", "user-rotation", "free"])?;
            crate::process::answer(adb(&serial, &["emu", "sensor", "set", "acceleration", gravity(turned)])?)?;
        }
        Action::Location(lat, lon) => {
            anyhow::ensure!(lat.abs() <= 90. && lon.abs() <= 180., "The latitude or longitude is out of range");
            crate::process::answer(adb(&serial, &["emu", "geo", "fix", &lon.to_string(), &lat.to_string()])?)?;
        }
        Action::Permission { app, permission, decision } => {
            anyhow::ensure!(*decision != Decision::Reset, "Android has no reset for a permission");
            // `pm` refuses a permission the app doesn't declare, and Photos names two: it fails only if none took
            let tried: Vec<_> = android_permissions(permission).iter().map(|p| sh(&["pm", decision.verb(), app, p])).collect();
            if !tried.iter().any(Result::is_ok) {
                tried.into_iter().find_map(Result::err).map_or_else(|| anyhow::bail!("Android has no {permission} permission"), Err)?;
            }
        }
        Action::OpenUrl(url) => sh(&["am", "start", "-a", "android.intent.action.VIEW", "-d", url])?,
        Action::Launch(app) => sh(&["monkey", "-p", app, "-c", "android.intent.category.LAUNCHER", "1"])?,
        Action::Terminate(app) => sh(&["am", "force-stop", app])?,
        Action::Toggle(toggle, _) => anyhow::bail!("{} is iOS's", toggle.label()),
        Action::LiquidGlass(_) | Action::ColorFilter(_) | Action::ClearLocation | Action::Push { .. } => anyhow::bail!("Android has no such setting"),
    }
    Ok(())
}

pub(super) fn elements(id: &str) -> anyhow::Result<Vec<[f64; 4]>> {
    let serial = serial(id)?;
    let out = adb(&serial, &["exec-out", "uiautomator", "dump", "/dev/tty"])?;
    let dump = String::from_utf8_lossy(&out.stdout);
    let frames = node_bounds(&dump);
    anyhow::ensure!(!frames.is_empty(), "uiautomator found no elements: {}", String::from_utf8_lossy(&out.stderr).trim());
    // `wm size` is the natural orientation's; the dump is in the display's as it's turned now
    let (mut w, mut h) = display_size(&serial).map_or((1., 1.), |(w, h)| (w as f64, h as f64));
    if frames.iter().any(|f| f[0] + f[2] > w + 1.) {
        (w, h) = (h, w);
    }
    Ok(frames.into_iter().map(|f| [f[0] / w, f[1] / h, f[2] / w, f[3] / h]).collect())
}

/// Each node's `bounds="[x1,y1][x2,y2]"` in a uiautomator dump, as `[x, y, width, height]` in pixels.
fn node_bounds(dump: &str) -> Vec<[f64; 4]> {
    let mut frames = Vec::new();
    for (_, rest) in dump.match_indices("bounds=\"").map(|(i, m)| (i, &dump[i + m.len()..])) {
        let Some(text) = rest.split('"').next() else { continue };
        let n: Vec<f64> = text.split(['[', ']', ',']).filter(|p| !p.is_empty()).filter_map(|p| p.parse().ok()).collect();
        if let [x1, y1, x2, y2] = n[..]
            && x2 > x1
            && y2 > y1
        {
            frames.push([x1, y1, x2 - x1, y2 - y1]);
        }
    }
    frames
}

/// A foldable's hinge angle, from its sensor; `None` for a phone that has none.
fn hinge_angle(serial: &str) -> Option<f64> {
    let out = adb(serial, &["emu", "sensor", "get", "hinge-angle0"]).ok()?;
    hinge_reading(&String::from_utf8_lossy(&out.stdout))
}

/// `hinge-angle0 = 180.000000` (`KO: …` where there's no hinge).
fn hinge_reading(text: &str) -> Option<f64> {
    text.split("hinge-angle0").nth(1)?.trim_start().strip_prefix('=')?.split_whitespace().next()?.parse().ok()
}

/// Folds the emulator shut or open, and waits for its hinge to get there; the angle it ended at.
fn fold(serial: &str, shut: bool) -> anyhow::Result<f64> {
    let out = adb(serial, &["emu", if shut { "fold" } else { "unfold" }])?;
    let said = String::from_utf8_lossy(&out.stdout);
    anyhow::ensure!(out.status.success() && !said.contains("KO:"), "The emulator would not fold: {}", said.trim());
    let want = if shut { 0. } else { 180. };
    for _ in 0..40 {
        if let Some(angle) = hinge_angle(serial).filter(|a| (a - want).abs() < 1.) {
            return Ok(angle);
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    anyhow::bail!("The emulator did not confirm the fold")
}

/// The display's size now, as `wm size` says (its override, if one is set): a foldable's changes as it folds.
fn display_size(serial: &str) -> Option<(u32, u32)> {
    let out = adb(serial, &["shell", "wm", "size"]).ok()?;
    let text = String::from_utf8_lossy(&out.stdout);
    let size = |line: &str| {
        let (w, h) = line.split(':').nth(1)?.trim().split_once('x')?;
        Some((w.parse().ok()?, h.parse().ok()?))
    };
    let line = |label: &str| text.lines().find(|l| l.starts_with(label)).and_then(size);
    line("Override size").or_else(|| line("Physical size"))
}

/// A folded foldable shows its cover screen: the emulators' closed posture is under 30°.
fn folded(angle: f64) -> bool {
    angle < 30.
}

pub(super) fn screenshot(id: &str, to: &Path) -> anyhow::Result<()> {
    let out = adb(&serial(id)?, &["exec-out", "screencap", "-p"])?;
    // a foldable's has a warning ahead of the image (several displays, none named)
    let png = out.stdout.windows(4).position(|w| w == b"\x89PNG").map(|at| &out.stdout[at..]);
    let png = png.filter(|_| out.status.success()).ok_or_else(|| anyhow::anyhow!("screencap failed: {}", String::from_utf8_lossy(&out.stderr).trim()))?;
    Ok(std::fs::write(to, png)?)
}

#[derive(Clone)]
struct Client {
    grpc: tonic::client::Grpc<tonic::transport::Channel>,
    token: String,
}

impl Client {
    fn request<M>(&self, message: M) -> tonic::Request<M> {
        let mut request = tonic::Request::new(message);
        if let Ok(value) = format!("Bearer {}", self.token).parse() {
            request.metadata_mut().insert("authorization", value);
        }
        request
    }

    fn path(method: &str) -> PathAndQuery {
        PathAndQuery::try_from(format!("{SERVICE}{method}")).expect("a gRPC path")
    }

    async fn send<M: prost::Message + Send + Sync + 'static>(&mut self, method: &str, message: M) -> Result<(), tonic::Status> {
        self.grpc.ready().await.map_err(|e| tonic::Status::unavailable(e.to_string()))?;
        let request = self.request(message);
        self.grpc.unary::<M, Empty, _>(request, Self::path(method), tonic_prost::ProstCodec::default()).await.map(drop)
    }

    /// Frames until the stream ends, each into `feed`, or until the display's size changes (it's been folded).
    async fn stream(&mut self, native: &Mutex<(u32, u32)>, feed: &Feed, wake: &Wake) -> Result<(), tonic::Status> {
        self.grpc.ready().await.map_err(|e| tonic::Status::unavailable(e.to_string()))?;
        let now = || native.lock().map_or((1080, 2400), |n| *n);
        let asked = now();
        // ponytail: half the native size, plenty for a panel; a sharper stream if it ever looks soft. A square bound: a
        // turned screen's image fits it as well as an upright one's
        let side = asked.0.max(asked.1) / 2;
        let request = self.request(ImageFormat { format: 2, rotation: None, width: side, height: side });
        let codec = tonic_prost::ProstCodec::<ImageFormat, Image>::default();
        let mut frames = self.grpc.server_streaming(request, Self::path("streamScreenshot"), codec).await?.into_inner();
        while let Some(image) = frames.message().await? {
            if now() != asked {
                return Ok(());
            }
            let Some(format) = image.format else { continue };
            if image.image.len() == (format.width * format.height * 3) as usize {
                // touches are on the screen as it's mounted: a turned one's turn back (`Frame::device_point`)
                let turn = turn(format.rotation.map_or(0, |r| r.rotation));
                feed.put(Frame::new(format.width, format.height, image.image, true, None, turn));
                wake();
            }
        }
        Ok(())
    }
}

enum Input {
    Touch(TouchEvent),
    Key(KeyboardEvent),
}

pub struct Session {
    input: tokio::sync::mpsc::UnboundedSender<Input>,
    /// The screen's size in pixels, which touches are in; a foldable's changes as it folds.
    native: Arc<Mutex<(u32, u32)>>,
    serial: String,
    feed: Arc<Feed>,
    wake: Wake,
    tasks: [tokio::task::JoinHandle<()>; 2],
    /// A scroll's finger, while one is.
    drag: Arc<Mutex<Option<Drag>>>,
}

/// A scroll as a finger's drag: where it is (0 to 1 across the screen), whether it's down yet, when it last moved.
struct Drag {
    x: f64,
    y: f64,
    down: bool,
    at: Instant,
}

/// How long a mouse wheel rests before its finger is lifted; past Android's 40 ms, so the lift doesn't fling.
const WHEEL_HELD: Duration = Duration::from_millis(150);

/// A finger at `x`, `y`, from 0 to 1 across the screen, in the screen's pixels.
fn finger(native: &Mutex<(u32, u32)>, phase: Touch, x: f64, y: f64) -> Input {
    let native = native.lock().map_or((1080, 2400), |n| *n);
    let point = TouchPoint {
        x: (x.clamp(0., 1.) * native.0 as f64) as i32,
        y: (y.clamp(0., 1.) * native.1 as f64) as i32,
        identifier: 0,
        pressure: if phase == Touch::Up { 0 } else { 1 },
    };
    Input::Touch(TouchEvent { touches: vec![point] })
}

impl Session {
    pub(super) fn open(id: &str, rt: &tokio::runtime::Handle, feed: Arc<Feed>, wake: Wake) -> anyhow::Result<Session> {
        let live = running().remove(id).ok_or_else(|| anyhow::anyhow!("{id} is not running"))?;
        let angle = hinge_angle(&live.serial);
        let native = angle.and_then(|_| display_size(&live.serial)).or_else(|| lcd(&config(id))).unwrap_or((1080, 2400));
        let native = Arc::new(Mutex::new(native));
        if let (Some(angle), Ok(mut fold)) = (angle, feed.fold.lock()) {
            *fold = Some(Fold { duo: false, angle, cover: folded(angle), stand: None, orientation: "portrait", busy: false, error: None });
        }
        let url = format!("http://127.0.0.1:{}", live.port);
        let channel = rt.block_on(tonic::transport::Endpoint::from_shared(url)?.connect())?;
        let client = Client { grpc: tonic::client::Grpc::new(channel).max_decoding_message_size(64 << 20), token: live.token };
        let mut frames = client.clone();
        let stream = rt.spawn({
            let (native, feed, wake) = (native.clone(), feed.clone(), wake.clone());
            async move {
                loop {
                    if let Err(e) = frames.stream(&native, &feed, &wake).await {
                        feed.fail(format!("The emulator's video stopped: {}", e.message()));
                        wake();
                        tokio::time::sleep(Duration::from_secs(1)).await;
                    }
                }
            }
        });
        let (input, mut queue) = tokio::sync::mpsc::unbounded_channel();
        let mut sender = client;
        // one queue, so a touch's down, moves and up land in order
        let send = rt.spawn(async move {
            while let Some(event) = queue.recv().await {
                let _ = match event {
                    Input::Touch(t) => sender.send("sendTouch", t).await,
                    Input::Key(k) => sender.send("sendKey", k).await,
                };
            }
        });
        Ok(Session { input, native, serial: live.serial, feed, wake, tasks: [stream, send], drag: Default::default() })
    }

    /// Folds or unfolds a foldable (it takes no other angle, nor stands), off this thread.
    pub(super) fn hinge(&self, hinge: Hinge) {
        let Hinge::Angle(angle) = hinge else { return };
        let (serial, native, feed, wake) = (self.serial.clone(), self.native.clone(), self.feed.clone(), self.wake.clone());
        feed.change_fold(|f| (f.busy, f.error) = (true, None));
        wake();
        std::thread::spawn(move || {
            let done = fold(&serial, folded(angle));
            if let (Some(size), Ok(mut n)) = (display_size(&serial), native.lock()) {
                *n = size;
            }
            feed.change_fold(|f| {
                f.busy = false;
                match done {
                    Ok(angle) => (f.angle, f.cover) = (angle, folded(angle)),
                    Err(e) => f.error = Some(format!("{e:#}")),
                }
            });
            wake();
        });
    }

    pub(super) fn touch(&self, phase: Touch, x: f64, y: f64) {
        let _ = self.input.send(finger(&self.native, phase, x, y));
    }

    /// A trackpad's scroll or a mouse wheel's, as a drag. The finger goes down on the first move, so fingers resting
    /// on the trackpad neither tap nor long-press; a wheel's is lifted once the wheel rests.
    pub(super) fn scroll(&self, phase: gpui::TouchPhase, x: f64, y: f64, dx: f64, dy: f64, wheel: bool) {
        let Ok(mut drag) = self.drag.lock() else { return };
        if phase == gpui::TouchPhase::Started || (wheel && drag.is_none()) {
            *drag = Some(Drag { x, y, down: false, at: Instant::now() });
            if wheel {
                self.lift_when_idle();
            }
        }
        // a trackpad's momentum, after its fingers left: Android flings on its own
        let Some(d) = drag.as_mut() else { return };
        match phase {
            gpui::TouchPhase::Ended | gpui::TouchPhase::Cancelled if !wheel => {
                if d.down {
                    self.touch(Touch::Up, d.x, d.y);
                }
                *drag = None;
            }
            _ if dx != 0. || dy != 0. => {
                if !d.down {
                    self.touch(Touch::Down, d.x, d.y);
                    d.down = true;
                }
                // ponytail: a finger at the screen's edge stops there; a long swipe scrolls on in the next one
                (d.x, d.y, d.at) = ((d.x + dx).clamp(0., 1.), (d.y + dy).clamp(0., 1.), Instant::now());
                self.touch(Touch::Move, d.x, d.y);
            }
            _ => {}
        }
    }

    /// Lifts a mouse wheel's finger once the wheel has rested `WHEEL_HELD`.
    fn lift_when_idle(&self) {
        let (drag, input, native) = (self.drag.clone(), self.input.clone(), self.native.clone());
        std::thread::spawn(move || loop {
            std::thread::sleep(WHEEL_HELD / 3);
            let Ok(mut held) = drag.lock() else { return };
            match held.as_ref() {
                Some(d) if d.at.elapsed() < WHEEL_HELD => continue,
                Some(d) if d.down => drop(input.send(finger(&native, Touch::Up, d.x, d.y))),
                _ => {}
            }
            *held = None;
            return;
        });
    }

    fn press(&self, key: &str, text: &str) {
        let _ = self.input.send(Input::Key(KeyboardEvent { event_type: 2, key: key.into(), text: text.into() }));
    }

    pub(super) fn button(&self, button: Button) {
        self.press(
            match button {
                Button::Home => "GoHome",
                Button::Back => "GoBack",
                Button::Recents => "AppSwitch",
            },
            "",
        );
    }

    pub(super) fn key(&self, name: &str) -> bool {
        super::w3c_key(name).map(|key| self.press(key, "")).is_some()
    }

    pub(super) fn text(&self, text: &str) {
        self.press("", text);
    }
}

impl Drop for Session {
    fn drop(&mut self) {
        self.tasks.iter().for_each(tokio::task::JoinHandle::abort);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scrolls_drag_a_finger_put_down_on_the_first_move() {
        use gpui::TouchPhase::*;
        let rt = tokio::runtime::Runtime::new().unwrap();
        let (input, mut sent) = tokio::sync::mpsc::unbounded_channel();
        let tasks = [rt.spawn(async {}), rt.spawn(async {})];
        let native = Arc::new(Mutex::new((1000, 2000)));
        let s = Session { input, native, serial: String::new(), feed: Default::default(), wake: Arc::new(|| {}), tasks, drag: Default::default() };
        let mut touches = || std::iter::from_fn(|| sent.try_recv().ok()).map(|i| match i {
            Input::Touch(t) => (t.touches[0].y, t.touches[0].pressure),
            Input::Key(_) => panic!("a key"),
        }).collect::<Vec<_>>();
        // fingers resting on the trackpad: nothing, so no tap
        s.scroll(Started, 0.5, 0.5, 0., 0., false);
        assert_eq!(touches(), []);
        s.scroll(Moved, 0.5, 0.5, 0., 0.1, false);
        assert_eq!(touches(), [(1000, 1), (1200, 1)]);
        s.scroll(Ended, 0.5, 0.5, 0., 0., false);
        assert_eq!(touches(), [(1200, 0)]);
        // momentum, after the fingers left
        s.scroll(Moved, 0.5, 0.5, 0., 0.1, false);
        assert_eq!(touches(), []);
        // a wheel's finger lifts once it rests
        s.scroll(Moved, 0.5, 0.5, 0., -0.1, true);
        assert_eq!(touches(), [(1000, 1), (800, 1)]);
        std::thread::sleep(WHEEL_HELD * 2);
        assert_eq!(touches(), [(800, 0)]);
    }

    #[test]
    fn hinge_sensor_reads() {
        assert_eq!(hinge_reading("hinge-angle0 = 180.000000\nOK\n"), Some(180.));
        assert_eq!(hinge_reading("KO: 'hinge-angle0' sensor is disabled.\n"), None);
    }

    #[test]
    fn tools_read_from_one_shell() {
        let out = "Night mode: yes\n@@\n1.15\n@@\n0.0\n@@\n1\n@@\n  mCurrentFocus=Window{8e3a1b u0 com.example.app/com.example.app.Main}\n  mFocusedApp=ActivityRecord{1 u0 com.example.app/.Main t9}\n";
        let s = read_answers(out);
        assert_eq!(s.dark, Some(true));
        assert_eq!(s.text_size, Some(tools::TextSize::Large));
        assert_eq!(s.toggles, [(Toggle::ReduceMotion, true), (Toggle::Network, true)]);
        assert_eq!(s.app.as_deref(), Some("com.example.app"));
        assert_eq!(read_answers(""), Settings::default());
        let fresh = read_answers("Night mode: no\n@@\nnull\n@@\nnull\n@@\n0\n@@\n");
        assert_eq!((fresh.dark, fresh.text_size), (Some(false), None));
        assert_eq!(fresh.toggles, [(Toggle::ReduceMotion, false), (Toggle::Network, false)]);
    }

    #[test]
    fn uiautomator_bounds_read() {
        let dump = r#"<node bounds="[0,0][1080,2400]"><node text="a" bounds="[10,20][110,70]" /><node bounds="[5,5][5,9]" /></node>"#;
        assert_eq!(node_bounds(dump), [[0., 0., 1080., 2400.], [10., 20., 100., 50.]]);
    }

    #[test]
    fn attached_devices_read() {
        let text = "List of devices attached\nR5CT123 device usb:1-1 product:x model:SM_S911B transport_id:2\nemulator-5554 device product:sdk model:sdk_gphone64 transport_id:1\nZY22 unauthorized usb:1-2 transport_id:3\n\n";
        assert_eq!(attached_lines(text), [("R5CT123".into(), "SM S911B".into()), ("emulator-5554".into(), "sdk gphone64".into())]);
    }

    #[test]
    fn avd_config_reads() {
        let c = ini("hw.lcd.width=1280\nhw.lcd.height = 2856\nimage.sysdir.1=system-images/android-36/google_apis_playstore/arm64-v8a/\n");
        assert_eq!(lcd(&c), Some((1280, 2856)));
        assert_eq!(api_level(&c["image.sysdir.1"]).as_deref(), Some("API 36"));
    }
}

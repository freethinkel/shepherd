//! Android emulators through their own gRPC (`emulator_controller.proto` in the SDK), as serve-emu's `grpc-screenshot`
//! source: raw RGB frames streamed on change, touches and keys sent back. A running emulator leaves a discovery file
//! with its gRPC port and token; booting is the SDK's `emulator`, headless, so the panel is its only window.
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
    #[prost(uint32, tag = "3")]
    width: u32,
    #[prost(uint32, tag = "4")]
    height: u32,
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
            Device { platform: Platform::Android, id: id.to_string(), name, version, running: live.contains_key(id) }
        })
        .collect();
    let check = match devices.is_empty() {
        true => Check { ready: false, message: "The Android SDK is installed, but no virtual device exists. Create one in Android Studio → Device Manager.".into() },
        false => Check { ready: true, message: "The Android SDK and Emulator are available.".into() },
    };
    (devices, check)
}

fn adb(serial: &str, args: &[&str]) -> anyhow::Result<std::process::Output> {
    let sdk = sdk().ok_or_else(|| anyhow::anyhow!("The Android SDK was not found"))?;
    Ok(crate::process::command(tool(&sdk, "platform-tools", "adb")).arg("-s").arg(serial).args(args).output()?)
}

fn serial(id: &str) -> anyhow::Result<String> {
    running().remove(id).map(|r| r.serial).ok_or_else(|| anyhow::anyhow!("{id} is not running"))
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
        // ponytail: half the native size, plenty for a panel; a sharper stream if it ever looks soft
        let request = self.request(ImageFormat { format: 2, width: asked.0 / 2, height: asked.1 / 2 });
        let codec = tonic_prost::ProstCodec::<ImageFormat, Image>::default();
        let mut frames = self.grpc.server_streaming(request, Self::path("streamScreenshot"), codec).await?.into_inner();
        while let Some(image) = frames.message().await? {
            if now() != asked {
                return Ok(());
            }
            let Some(format) = image.format else { continue };
            if image.image.len() == (format.width * format.height * 3) as usize {
                feed.put(Frame { width: format.width, height: format.height, pixels: image.image, rgb: true, screen: None });
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
        Ok(Session { input, native, serial: live.serial, feed, wake, tasks: [stream, send] })
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
        let native = self.native.lock().map_or((1080, 2400), |n| *n);
        let point = TouchPoint {
            x: (x.clamp(0., 1.) * native.0 as f64) as i32,
            y: (y.clamp(0., 1.) * native.1 as f64) as i32,
            identifier: 0,
            pressure: if phase == Touch::Up { 0 } else { 1 },
        };
        let _ = self.input.send(Input::Touch(TouchEvent { touches: vec![point] }));
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
    fn hinge_sensor_reads() {
        assert_eq!(hinge_reading("hinge-angle0 = 180.000000\nOK\n"), Some(180.));
        assert_eq!(hinge_reading("KO: 'hinge-angle0' sensor is disabled.\n"), None);
    }

    #[test]
    fn avd_config_reads() {
        let c = ini("hw.lcd.width=1280\nhw.lcd.height = 2856\nimage.sysdir.1=system-images/android-36/google_apis_playstore/arm64-v8a/\n");
        assert_eq!(lcd(&c), Some((1280, 2856)));
        assert_eq!(api_level(&c["image.sysdir.1"]).as_deref(), Some("API 36"));
    }
}

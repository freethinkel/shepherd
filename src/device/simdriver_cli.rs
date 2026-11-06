// Vendored from simdriver-cli 0.2.4 (crates.io, MIT, the same authors as vendor/simdriver): its `main.rs` as
// `shepherd device`, so agents drive a simulator without a second binary to install. Changed: `main` is `cli`, the
// messages name `shepherd-device`, and a command with no `--udid` takes the device open in the agent's workspace
// (`super::agent`) before the only booted one, and marks it as the agent's.
//! The simdriver command line: see the simulator's screen, act on it, and set it up —
//! one process per call. The agent layer's refs (`e4`) carry between calls through a
//! small state file per device, so `snapshot` then `tap e4` work as separate commands.
use std::path::PathBuf;
use std::time::Duration;

use simdriver::DeviceSelector;
use simdriver::{self as sd, Device};
use simdriver_agent::{Agent, Ended, RecordOptions, Recording, SnapshotOptions, Step, StepsReport};

const USAGE: &str = "Drive an iOS simulator from the command line.

Usage: shepherd-device <command> [arguments]

Devices:
  devices                                  List simulators, booted first.
  boot <udid> | shutdown <udid>
  apps                                     List installed apps.

The screen, through the agent layer:
  agent snapshot [--settle-ms N] [--grid] [--cols N] [--image <path>]
  agent tap <target> [--expect <text>]
  agent steps <json | @file> [--out <dir>]
  screenshot [<path>]
      The screen as a plain PNG (no element boxes, no grid). <path> is a file, or a
      directory to put it in; default ~/.cache/simdriver/shots/<udid>-<time>.png.
      Prints the path written.
  agent record [--time <dur>] [--idle-ms N] [--full] [--fps N] [--max-frames N]
               [--out <dir>] [-- tap <target> | -- steps <json | @file>]
      Record what the screen does around the actions after `--` (none: just watch).
      <dur> is 5s, 1.5s, 800ms or a bare number of ms (default 5s, at most 30s).
      Writes sheet-N.png, frame-<n>-<t>ms.png and record.json to --out (default
      ~/.cache/simdriver/records/<udid>-<time>/); the report on stdout lists them.
      An --out that cannot be written is refused before the simulator is touched; if a
      write still fails, the recording goes to a temporary directory instead (exit 1).
      A steps run whose first step is `record` is recorded the same way (same --out).

Apps and simulator state:
  launch <bundle> | terminate <bundle> | open-url <url>
  clipboard get | clipboard set <text>
  set permission <grant|revoke|reset> <service> <bundle>
  set content-size <size> | set appearance <light|dark>
  set location <lat> <lon> | set status-bar <override|clear>
  notify <bundle> <aps-json>
  reset app-data <bundle> | reset keychain

A target is a ref from the last snapshot (e4), a label on screen (\"Log in\"),
a grid cell (C4, tapped at its centre), or a point in screen points (@201,816).
The last snapshot is remembered per device between calls.

Options:
  --udid <udid>   Which simulator (default: the one open in this workspace's Device tab,
                  else the only booted one).
  --json          Machine-readable output.
  --fresh         agent: ignore the remembered snapshot.
";

pub fn cli(args: &[String]) -> i32 {
    match run(args) {
        Ok(()) => 0,
        Err(CliError::Usage(message)) => {
            if !message.is_empty() {
                eprintln!("shepherd-device: {message}");
            }
            eprint!("{USAGE}");
            2
        }
        Err(CliError::Fail(message)) => {
            eprintln!("shepherd-device: {message}");
            1
        }
    }
}

#[derive(Debug)]
enum CliError {
    Usage(String),
    Fail(String),
}

impl From<String> for CliError {
    fn from(message: String) -> Self {
        CliError::Fail(message)
    }
}

impl From<&str> for CliError {
    fn from(message: &str) -> Self {
        CliError::Fail(message.to_string())
    }
}

impl From<std::io::Error> for CliError {
    fn from(error: std::io::Error) -> Self {
        CliError::Fail(error.to_string())
    }
}

impl From<serde_json::Error> for CliError {
    fn from(error: serde_json::Error) -> Self {
        CliError::Fail(error.to_string())
    }
}

impl From<sd::Error> for CliError {
    fn from(error: sd::Error) -> Self {
        CliError::Fail(error.to_string())
    }
}

/// Flags that take a value; everything else `--flag`-shaped is boolean.
const VALUE_FLAGS: [&str; 10] = [
    "--udid",
    "--expect",
    "--image",
    "--cols",
    "--settle-ms",
    "--time",
    "--idle-ms",
    "--fps",
    "--max-frames",
    "--out",
];

/// Positional words and named flags, with unknown flags rejected.
struct Args {
    words: Vec<String>,
    flags: Vec<(String, String)>,
}

fn parse_args(args: &[String], allowed: &[&str]) -> Result<Args, CliError> {
    let mut words = Vec::new();
    let mut flags = Vec::new();
    let mut i = 0;
    while i < args.len() {
        let arg = &args[i];
        if arg == "--help" || arg == "-h" {
            return Err(CliError::Usage(String::new()));
        }
        if !arg.starts_with("--") {
            words.push(arg.clone());
            i += 1;
            continue;
        }
        if arg == "--json" || arg == "--fresh" {
            flags.push((arg.clone(), String::new()));
            i += 1;
            continue;
        }
        if !allowed.contains(&arg.as_str()) {
            return Err(CliError::Usage(format!("unknown option {arg}")));
        }
        if VALUE_FLAGS.contains(&arg.as_str()) {
            let value = args
                .get(i + 1)
                .ok_or_else(|| CliError::Usage(format!("{arg} needs a value")))?;
            flags.push((arg.clone(), value.clone()));
            i += 2;
        } else {
            flags.push((arg.clone(), String::new()));
            i += 1;
        }
    }
    Ok(Args { words, flags })
}

impl Args {
    fn flag(&self, name: &str) -> Option<&str> {
        self.flags
            .iter()
            .find(|(key, _)| key == name)
            .map(|(_, value)| value.as_str())
    }
    fn bool(&self, name: &str) -> bool {
        self.flags.iter().any(|(key, _)| key == name)
    }
    fn word(&self, index: usize, what: &str) -> Result<String, CliError> {
        self.words
            .get(index)
            .cloned()
            .ok_or_else(|| CliError::Usage(format!("missing {what}")))
    }
}

fn run(args: &[String]) -> Result<(), CliError> {
    let command = args.first().ok_or_else(|| CliError::Usage(String::new()))?;
    let rest = &args[1..];
    match command.as_str() {
        "devices" => {
            let args = parse_args(rest, &[])?;
            let devices = sd::list_devices()?;
            if args.bool("--json") {
                let list: Vec<_> = devices
                    .iter()
                    .map(|d| serde_json::json!({"udid": d.udid, "name": d.name, "state": d.state, "deviceType": d.device_type, "runtime": d.runtime}))
                    .collect();
                println!("{}", serde_json::to_string_pretty(&list)?);
            } else {
                for device in devices {
                    println!("{:<10} {:<38} {}", device.state, device.udid, device.name);
                }
            }
            Ok(())
        }
        "boot" | "shutdown" => {
            let args = parse_args(rest, &[])?;
            let udid = args.word(0, "a device udid")?;
            if command == "boot" {
                sd::boot(&udid)?
            } else {
                sd::shutdown(&udid)?
            }
            println!("{command} {udid}");
            Ok(())
        }
        "apps" => {
            let args = parse_args(rest, &["--udid"])?;
            let udid = pick_booted(args.flag("--udid"))?;
            let apps = sd::list_apps(&udid)?;
            if args.bool("--json") {
                let list: Vec<_> = apps
                    .iter()
                    .map(|a| serde_json::json!({"bundleId": a.bundle_id, "name": a.name, "appType": a.app_type}))
                    .collect();
                println!("{}", serde_json::to_string_pretty(&list)?);
            } else {
                for app in apps {
                    println!("{:<40} {:<12} {}", app.bundle_id, app.app_type, app.name);
                }
            }
            Ok(())
        }
        "agent" => agent_command(rest),
        "screenshot" => {
            let args = parse_args(rest, &["--udid"])?;
            if let Some(extra) = args.words.get(1) {
                return Err(CliError::Usage(format!("unexpected {extra}")));
            }
            let udid = pick_booted(args.flag("--udid"))?;
            let path = screenshot_path(args.words.first().map(String::as_str), &udid)?;
            let driver = start_driver(&udid)?;
            let agent = Agent::new(driver);
            let shot = agent.screenshot().map_err(|e| e.to_string());
            agent.device().stop();
            let png = shot?;
            std::fs::write(&path, &png)
                .map_err(|e| format!("could not write {}: {e}", path.display()))?;
            if args.bool("--json") {
                let (width, height) = png_size(&png);
                println!(
                    "{}",
                    serde_json::json!({"path": path, "width": width, "height": height})
                );
            } else {
                println!("{}", path.display());
            }
            Ok(())
        }
        "launch" | "terminate" | "open-url" => {
            let args = parse_args(rest, &["--udid"])?;
            let udid = pick_booted(args.flag("--udid"))?;
            let what = args.word(0, "an argument")?;
            match command.as_str() {
                "launch" => sd::launch(&udid, &what)?,
                "terminate" => sd::terminate(&udid, &what)?,
                _ => sd::open_url(&udid, &what)?,
            }
            println!("{command} {what}");
            Ok(())
        }
        "clipboard" => {
            let args = parse_args(rest, &["--udid"])?;
            let udid = pick_booted(args.flag("--udid"))?;
            match args.word(0, "get or set")?.as_str() {
                "get" => {
                    let text = sd::clipboard_read(&udid)?;
                    println!("{text}");
                    Ok(())
                }
                "set" => {
                    let text = args.word(1, "the text")?;
                    sd::clipboard_write(&udid, &text)?;
                    println!("clipboard set");
                    Ok(())
                }
                _ => Err(CliError::Usage(
                    "clipboard get, or clipboard set <text>".into(),
                )),
            }
        }
        "set" => set_command(rest),
        "notify" => {
            let args = parse_args(rest, &["--udid"])?;
            let udid = pick_booted(args.flag("--udid"))?;
            let bundle = args.word(0, "a bundle id")?;
            let payload = args.word(1, "the APNs JSON payload")?;
            let value: serde_json::Value = serde_json::from_str(&payload)
                .map_err(|e| format!("the payload is not JSON: {e}"))?;
            if !value.is_object() {
                return Err("the payload must be a JSON object".into());
            }
            let path = state_dir()?.join(format!("push-{}.json", std::process::id()));
            std::fs::write(&path, value.to_string())?;
            let result = sd::push_notification(&udid, &bundle, &path.to_string_lossy());
            let _ = std::fs::remove_file(&path);
            result?;
            println!("notified {bundle}");
            Ok(())
        }
        "reset" => {
            let args = parse_args(rest, &["--udid"])?;
            let udid = pick_booted(args.flag("--udid"))?;
            match args.word(0, "app-data or keychain")?.as_str() {
                "app-data" => {
                    let bundle = args.word(1, "a bundle id")?;
                    sd::clear_app_data(&udid, &bundle)?;
                    println!("erased {bundle}'s data");
                    Ok(())
                }
                "keychain" => {
                    sd::reset_keychain(&udid)?;
                    println!("reset the keychain");
                    Ok(())
                }
                _ => Err(CliError::Usage(
                    "reset app-data <bundle>, or reset keychain".into(),
                )),
            }
        }
        _ => Err(CliError::Usage(format!("unknown command {command}"))),
    }
}

fn set_command(rest: &[String]) -> Result<(), CliError> {
    let args = parse_args(rest, &["--udid"])?;
    let udid = pick_booted(args.flag("--udid"))?;
    match args.word(0, "what to set")?.as_str() {
        "permission" => {
            let op = args.word(1, "grant, revoke or reset")?;
            let service = args.word(2, "a service")?;
            let bundle = args.word(3, "a bundle id")?;
            match op.as_str() {
                "grant" => sd::grant_permission(&udid, &service, &bundle)?,
                "revoke" => sd::revoke_permission(&udid, &service, &bundle)?,
                "reset" => sd::reset_permission(&udid, &service, &bundle)?,
                _ => {
                    return Err(CliError::Usage(
                        "permission takes grant, revoke or reset".into(),
                    ));
                }
            }
            println!("{op} {service} for {bundle}");
            Ok(())
        }
        "content-size" => {
            let size = args.word(1, "a size")?;
            sd::set_content_size(&udid, &size)?;
            println!("content size {size}");
            Ok(())
        }
        "appearance" => {
            let which = args.word(1, "light or dark")?;
            let appearance = match which.as_str() {
                "light" => sd::Appearance::Light,
                "dark" => sd::Appearance::Dark,
                _ => return Err(CliError::Usage("appearance takes light or dark".into())),
            };
            sd::set_appearance(&udid, appearance)?;
            println!("appearance {which}");
            Ok(())
        }
        "location" => {
            let lat: f64 = args
                .word(1, "a latitude")?
                .parse()
                .map_err(|_| "latitude must be a number")?;
            let lon: f64 = args
                .word(2, "a longitude")?
                .parse()
                .map_err(|_| "longitude must be a number")?;
            sd::set_location(&udid, lat, lon)?;
            println!("location {lat}, {lon}");
            Ok(())
        }
        "status-bar" => {
            match args.word(1, "override or clear")?.as_str() {
                "override" => sd::status_bar_override(&udid)?,
                "clear" => sd::status_bar_clear(&udid)?,
                _ => return Err(CliError::Usage("status-bar takes override or clear".into())),
            }
            println!("status bar {}", args.word(1, "")?);
            Ok(())
        }
        _ => Err(CliError::Usage(
            "set permission|content-size|appearance|location|status-bar".into(),
        )),
    }
}

fn agent_command(rest: &[String]) -> Result<(), CliError> {
    let action = rest
        .first()
        .ok_or_else(|| CliError::Usage("agent snapshot|tap|steps|record".into()))?;
    let rest = &rest[1..];
    match action.as_str() {
        "snapshot" => {
            let args = parse_args(rest, &["--udid", "--settle-ms", "--cols", "--image"])?;
            let udid = pick_booted(args.flag("--udid"))?;
            let driver = start_driver(&udid)?;
            let agent = Agent::new(driver);
            if !args.bool("--fresh") {
                restore_state(&agent, &udid);
            }
            let mut opts = SnapshotOptions {
                image: args.flag("--image").is_some(),
                ..SnapshotOptions::default()
            };
            if let Some(ms) = args.flag("--settle-ms") {
                opts.settle = Duration::from_millis(
                    ms.parse()
                        .map_err(|_| "settle-ms must be a number of milliseconds")?,
                );
            }
            if let Some(cols) = args.flag("--cols") {
                opts.cols = cols.parse().map_err(|_| "cols must be a number")?;
            }
            opts.grid = args.bool("--grid");
            let snapshot = agent.snapshot(&opts).map_err(|e| e.to_string())?;
            save_state(&agent, &udid);
            if let (Some(path), Some(png)) = (args.flag("--image"), &snapshot.image) {
                std::fs::write(path, png)?;
            }
            if args.bool("--json") {
                println!("{}", serde_json::to_string_pretty(&snapshot)?);
            } else {
                println!("{}", snapshot.text);
                if let Some(path) = args.flag("--image") {
                    if snapshot.image.is_some() {
                        println!("image: {path}");
                    } else {
                        println!("image: none (no frame was available)");
                    }
                }
            }
            agent.device().stop();
            Ok(())
        }
        "tap" => {
            let args = parse_args(rest, &["--udid", "--expect"])?;
            let udid = pick_booted(args.flag("--udid"))?;
            let target = args.word(0, "a target: e4, a label, or a grid cell")?;
            let driver = start_driver(&udid)?;
            let agent = Agent::new(driver);
            if !args.bool("--fresh") {
                restore_state(&agent, &udid);
            }
            let tapped = agent
                .tap(&target, args.flag("--expect"))
                .map_err(|e| e.to_string())?;
            save_state(&agent, &udid);
            if args.bool("--json") {
                println!("{}", serde_json::to_string(&tapped)?);
            } else {
                println!(
                    "tapped {} \"{}\" at ({}, {})",
                    tapped.target, tapped.name, tapped.at[0], tapped.at[1]
                );
            }
            agent.device().stop();
            Ok(())
        }
        "steps" => {
            let args = parse_args(rest, &["--udid", "--out"])?;
            let udid = pick_booted(args.flag("--udid"))?;
            let input = args.word(0, "a JSON array of steps, or @file")?;
            let steps = parse_steps_input(&input)?;
            // Only a run that records writes files; a misplaced `record` fails the run.
            let records = steps
                .first()
                .is_some_and(|s| matches!(s, Step::Record { .. }));
            let out = if records {
                Some(prepare_record_dir(
                    args.flag("--out").map(std::path::Path::new),
                    &udid,
                )?)
            } else {
                None
            };
            let driver = start_driver(&udid)?;
            let agent = Agent::new(driver);
            if !args.bool("--fresh") {
                restore_state(&agent, &udid);
            }
            let report = agent.steps(&steps);
            finish_run(&agent, &udid, report, out)
        }
        "record" => {
            let (head, tail) = split_double_dash(rest);
            let args = parse_args(
                head,
                &[
                    "--udid",
                    "--time",
                    "--idle-ms",
                    "--full",
                    "--fps",
                    "--max-frames",
                    "--out",
                ],
            )?;
            if let Some(extra) = args.words.first() {
                return Err(CliError::Usage(format!(
                    "unexpected {extra}; put the actions after --"
                )));
            }
            let opts = record_options(&args)?;
            let steps = parse_record_tail(tail)?;
            let udid = pick_booted(args.flag("--udid"))?;
            let out = Some(prepare_record_dir(
                args.flag("--out").map(std::path::Path::new),
                &udid,
            )?);
            let driver = start_driver(&udid)?;
            let agent = Agent::new(driver);
            if !args.bool("--fresh") {
                restore_state(&agent, &udid);
            }
            let report = agent.record(opts, &steps);
            finish_run(&agent, &udid, report, out)
        }
        _ => Err(CliError::Usage("agent snapshot|tap|steps|record".into())),
    }
}

/// The steps from a JSON array or `@file`.
fn parse_steps_input(input: &str) -> Result<Vec<Step>, CliError> {
    let json = if let Some(file) = input.strip_prefix('@') {
        std::fs::read_to_string(file).map_err(|e| format!("could not read {file}: {e}"))?
    } else {
        input.to_string()
    };
    Ok(serde_json::from_str(&json).map_err(|e| format!("the steps are not valid: {e}"))?)
}

/// Arguments before and after the first `--`.
fn split_double_dash(args: &[String]) -> (&[String], &[String]) {
    match args.iter().position(|a| a == "--") {
        Some(i) => (&args[..i], &args[i + 1..]),
        None => (args, &[]),
    }
}

/// `5s`, `1.5s`, `800ms`, or a bare number of milliseconds.
fn parse_duration(text: &str) -> Result<Duration, CliError> {
    let bad = || {
        CliError::Usage(format!(
            "{text:?} is not a duration; use 5s, 1.5s, 800ms or ms"
        ))
    };
    let (number, unit_ms) = if let Some(n) = text.strip_suffix("ms") {
        (n, 1.0)
    } else if let Some(n) = text.strip_suffix('s') {
        (n, 1000.0)
    } else {
        (text, 1.0)
    };
    if number.is_empty() || !number.chars().all(|c| c.is_ascii_digit() || c == '.') {
        return Err(bad());
    }
    let value: f64 = number.parse().map_err(|_| bad())?;
    if !value.is_finite() {
        return Err(bad());
    }
    Ok(Duration::from_millis((value * unit_ms).round() as u64))
}

fn record_options(args: &Args) -> Result<RecordOptions, CliError> {
    let mut opts = RecordOptions::default();
    if let Some(t) = args.flag("--time") {
        opts.time = parse_duration(t)?;
    }
    if let Some(ms) = args.flag("--idle-ms") {
        opts.idle =
            Duration::from_millis(ms.parse().map_err(|_| {
                CliError::Usage("--idle-ms must be a number of milliseconds".into())
            })?);
    }
    if let Some(n) = args.flag("--fps") {
        opts.fps = n
            .parse()
            .map_err(|_| CliError::Usage("--fps must be a number".into()))?;
    }
    if let Some(n) = args.flag("--max-frames") {
        opts.max_frames = n
            .parse()
            .map_err(|_| CliError::Usage("--max-frames must be a number".into()))?;
    }
    opts.full = args.bool("--full");
    Ok(opts)
}

/// What follows `--` in `agent record`: nothing, `tap <target>`, or `steps <json|@file>`.
fn parse_record_tail(tail: &[String]) -> Result<Vec<Step>, CliError> {
    let Some(command) = tail.first() else {
        return Ok(Vec::new());
    };
    let rest = &tail[1..];
    match command.as_str() {
        "tap" => {
            let args = parse_args(rest, &["--expect"])?;
            if args.flag("--expect").is_some() {
                return Err(CliError::Usage(
                    "--expect is not supported under record; use steps with an expect step".into(),
                ));
            }
            let target = args.word(0, "a target: e4, a label, or a grid cell")?;
            Ok(vec![Step::Tap { target }])
        }
        "steps" => {
            let args = parse_args(rest, &[])?;
            parse_steps_input(&args.word(0, "a JSON array of steps, or @file")?)
        }
        other => Err(CliError::Usage(format!(
            "after -- expected tap or steps, not {other}"
        ))),
    }
}

/// `frame-<index:02>-<t_ms>ms.png`.
fn frame_file_name(index: usize, t_ms: u64) -> String {
    format!("frame-{index:02}-{t_ms}ms.png")
}

fn unix_ms() -> u128 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0)
}

/// Write the sheets, frames and `record.json` into `dir` (created if missing; same-named
/// files are overwritten, nothing is deleted), filling in the absolute paths on `recording`.
fn write_recording(dir: &std::path::Path, recording: &mut Recording) -> Result<(), CliError> {
    std::fs::create_dir_all(dir).map_err(|e| format!("could not create {}: {e}", dir.display()))?;
    let dir = std::path::absolute(dir)?;
    let path_text = |name: &str| dir.join(name).to_string_lossy().into_owned();
    recording.sheet_files.clear();
    for (i, sheet) in recording.sheets.iter().enumerate() {
        let name = format!("sheet-{}.png", i + 1);
        std::fs::write(dir.join(&name), sheet)?;
        recording.sheet_files.push(path_text(&name));
    }
    for frame in &mut recording.frames {
        let name = frame_file_name(frame.index, frame.t_ms);
        std::fs::write(dir.join(&name), &frame.png)?;
        frame.file = Some(path_text(&name));
    }
    std::fs::write(
        dir.join("record.json"),
        serde_json::to_string_pretty(&recording)?,
    )?;
    Ok(())
}

/// Where `screenshot` writes: `path` as given, or a generated `<udid>-<unix_ms>.png` inside
/// it when it is a directory (existing, or written with a trailing `/`), or by default
/// inside `<cache>/simdriver/shots/`. Parent directories are created before the driver
/// starts, so a bad path costs nothing on the simulator. The result is absolute.
fn screenshot_path(path: Option<&str>, udid: &str) -> Result<PathBuf, CliError> {
    let name = || format!("{}-{}.png", udid_short(udid), unix_ms());
    let file = match path {
        Some("") => return Err(CliError::Usage("the screenshot path is empty".into())),
        Some(path) if path.ends_with('/') || std::path::Path::new(path).is_dir() => {
            PathBuf::from(path).join(name())
        }
        Some(path) => PathBuf::from(path),
        None => state_dir()?.join("shots").join(name()),
    };
    if let Some(parent) = file.parent().filter(|p| !p.as_os_str().is_empty()) {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("could not create {}: {e}", parent.display()))?;
    }
    Ok(std::path::absolute(&file)?)
}

/// A PNG's width and height, from its IHDR chunk.
fn png_size(png: &[u8]) -> (u32, u32) {
    let at = |i: usize| {
        png.get(i..i + 4)
            .map_or(0, |b| u32::from_be_bytes([b[0], b[1], b[2], b[3]]))
    };
    (at(16), at(20))
}

/// The default output directory: `<cache>/simdriver/records/<udid>-<unix_ms>/`.
fn default_record_dir(udid: &str) -> Result<PathBuf, CliError> {
    Ok(state_dir()?
        .join("records")
        .join(format!("{}-{}", udid_short(udid), unix_ms())))
}

/// Where a recording goes when its directory cannot be written:
/// `<tmp>/simdriver-record-<udid>-<unix_ms>/`.
fn fallback_record_dir(udid: &str) -> PathBuf {
    std::env::temp_dir().join(format!(
        "simdriver-record-{}-{}",
        udid_short(udid),
        unix_ms()
    ))
}

/// A recording's directory, ready before the run.
#[derive(Debug)]
struct RecordDir {
    /// Absolute.
    path: PathBuf,
    /// It did not exist before (so an unused one can be removed again).
    created: bool,
}

/// The recording's directory (`out`, or the default), created and proven writable before
/// the driver starts, so a bad `--out` costs nothing on the simulator.
fn prepare_record_dir(out: Option<&std::path::Path>, udid: &str) -> Result<RecordDir, CliError> {
    let dir = match out {
        Some(path) if path.as_os_str().is_empty() => {
            return Err(CliError::Usage("--out needs a directory".into()));
        }
        Some(path) => path.to_path_buf(),
        None => default_record_dir(udid)?,
    };
    let refuse = |e: &dyn std::fmt::Display| {
        CliError::Fail(format!(
            "cannot write the recording to {}: {e}",
            dir.display()
        ))
    };
    let existed = dir.exists();
    if existed && !dir.is_dir() {
        return Err(refuse(&"it exists and is not a directory"));
    }
    std::fs::create_dir_all(&dir).map_err(|e| refuse(&e))?;
    let probe = dir.join(format!(".simdriver-probe-{}", std::process::id()));
    std::fs::write(&probe, b"").map_err(|e| refuse(&e))?;
    let _ = std::fs::remove_file(&probe);
    Ok(RecordDir {
        path: std::path::absolute(&dir).map_err(|e| refuse(&e))?,
        created: !existed,
    })
}

/// Write the recording into `target`; if that fails, into `fallback` instead, so a
/// recording is never lost to a write failure. `None` when `target` took it; otherwise
/// the error, saying where it was written (or that it could not be written at all, with
/// no file paths left on the recording).
fn save_recording(
    target: &std::path::Path,
    fallback: &std::path::Path,
    recording: &mut Recording,
) -> Option<String> {
    let Err(first) = write_recording(target, recording) else {
        return None;
    };
    let first = format!(
        "could not write the recording to {}: {}",
        target.display(),
        message(first)
    );
    match write_recording(fallback, recording) {
        Ok(()) => Some(format!(
            "{first}; wrote it to {} instead",
            fallback.display()
        )),
        Err(second) => {
            recording.sheet_files.clear();
            for frame in &mut recording.frames {
                frame.file = None;
            }
            Some(format!(
                "{first}; nor to {}: {}",
                fallback.display(),
                message(second)
            ))
        }
    }
}

fn message(e: CliError) -> String {
    match e {
        CliError::Usage(m) | CliError::Fail(m) => m,
    }
}

/// Save state, write any recording (first, so nothing below can lose it), then any
/// screenshots, print the report, stop the driver, and turn a failed run into an error.
fn finish_run(
    agent: &Agent<sd::Driver>,
    udid: &str,
    mut report: StepsReport,
    out: Option<RecordDir>,
) -> Result<(), CliError> {
    save_state(agent, udid);
    let mut errors: Vec<String> = Vec::new();
    if let Some(recording) = report.record.as_mut() {
        let fallback = fallback_record_dir(udid);
        let target = out.map_or_else(|| fallback.clone(), |d| d.path);
        errors.extend(save_recording(&target, &fallback, recording));
    } else if let Some(dir) = out.filter(|d| d.created) {
        // Made for a recording that never started: remove it again if still empty.
        let _ = std::fs::remove_dir(dir.path);
    }
    let mut value = serde_json::to_value(&report)?;
    if !report.screenshots.is_empty() {
        match state_dir() {
            Ok(dir) => {
                if let Some(shots) = value.get_mut("screenshots").and_then(|v| v.as_array_mut()) {
                    for (index, (shot, taken)) in
                        shots.iter_mut().zip(&report.screenshots).enumerate()
                    {
                        if let Some(file) = &taken.file {
                            // A `screenshot` step with a `path` already wrote it there.
                            shot["png"] = serde_json::Value::String(file.clone());
                            continue;
                        }
                        let path = dir.join(format!(
                            "shot-{}-{index}-{}.png",
                            udid_short(udid),
                            taken.target.replace(['/', ' ', '"'], "_")
                        ));
                        match std::fs::write(&path, &taken.png) {
                            Ok(()) => {
                                shot["png"] =
                                    serde_json::Value::String(path.to_string_lossy().into());
                            }
                            Err(e) => {
                                errors.push(format!("could not write {}: {e}", path.display()));
                                shot["png"] = serde_json::Value::Null;
                            }
                        }
                    }
                }
            }
            Err(e) => {
                errors.push(format!("could not write the screenshots: {}", message(e)));
                if let Some(shots) = value.get_mut("screenshots").and_then(|v| v.as_array_mut()) {
                    for shot in shots {
                        shot["png"] = serde_json::Value::Null;
                    }
                }
            }
        }
    }
    println!("{}", serde_json::to_string_pretty(&value)?);
    agent.device().stop();
    if let Some(f) = report.failed.as_ref().filter(|_| !report.ok) {
        errors.push(format!("step {} failed: {}", f.index, f.error));
    }
    if let Some(recording) = report.record.as_ref().filter(|r| r.ended == Ended::Error) {
        errors.push(format!(
            "the recording ended in error: {}",
            recording.error.clone().unwrap_or_default()
        ));
    }
    if errors.is_empty() {
        Ok(())
    } else {
        Err(CliError::Fail(errors.join("\n")))
    }
}

/// The udid to use: the one asked for, or the single booted device — with the devices
/// listed when it is ambiguous, so the next command can be copied.
fn pick_booted(udid: Option<&str>) -> Result<String, CliError> {
    let udid = pick(udid)?;
    super::agent::acting(&udid);
    Ok(udid)
}

fn pick(udid: Option<&str>) -> Result<String, CliError> {
    if let Some(udid) = udid {
        if udid.is_empty() {
            return Err(CliError::Usage("--udid needs a value".into()));
        }
        return Ok(udid.to_string());
    }
    if let Some(udid) = super::agent::workspace_device(super::Platform::Ios) {
        return Ok(udid);
    }
    let booted: Vec<Device> = sd::list_devices()?
        .into_iter()
        .filter(|device| device.state == "Booted")
        .collect();
    match booted.as_slice() {
        [one] => Ok(one.udid.clone()),
        [] => Err("no simulator is booted; boot one with: shepherd-device boot <udid>".into()),
        many => {
            let list = many
                .iter()
                .map(|device| format!("  {}  {}", device.udid, device.name))
                .collect::<Vec<_>>()
                .join("\n");
            Err(format!("more than one simulator is booted; pass --udid:\n{list}").into())
        }
    }
}

fn start_driver(udid: &str) -> Result<sd::Driver, CliError> {
    let options = sd::Options {
        device: DeviceSelector::Udid(udid.to_string()),
        ..sd::Options::default()
    };
    Ok(sd::Driver::start(options, |_| {})?)
}

fn state_dir() -> Result<PathBuf, CliError> {
    let base = std::env::var_os("XDG_CACHE_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".cache")))
        .ok_or("could not find a cache directory (no HOME)")?;
    let dir = base.join("simdriver");
    std::fs::create_dir_all(&dir)
        .map_err(|e| format!("could not create {}: {e}", dir.display()))?;
    Ok(dir)
}

fn state_file(udid: &str) -> Result<PathBuf, CliError> {
    Ok(state_dir()?.join(format!("agent-{}.json", udid_short(udid))))
}

fn udid_short(udid: &str) -> String {
    udid.chars().take(8).collect()
}

fn restore_state(agent: &Agent<sd::Driver>, udid: &str) {
    let Ok(path) = state_file(udid) else { return };
    let Ok(text) = std::fs::read_to_string(&path) else {
        return;
    };
    // A state from another version is ignored: start fresh rather than fail.
    if let Ok(state) = serde_json::from_str(&text) {
        agent.restore(state);
    }
}

fn save_state(agent: &Agent<sd::Driver>, udid: &str) {
    let Ok(path) = state_file(udid) else { return };
    let Ok(json) = serde_json::to_string(&agent.state()) else {
        return;
    };
    let _ = std::fs::write(path, json);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn screenshot_path_is_a_file_or_a_name_in_a_directory() {
        let dir = std::env::temp_dir().join(format!(
            "simdriver-cli-shot-{}-{}",
            std::process::id(),
            unix_ms()
        ));
        let file = dir.join("a").join("shot.png");
        let got = screenshot_path(file.to_str(), "ABCDEF12-3456").unwrap();
        assert_eq!(got, file);
        assert!(dir.join("a").is_dir());

        let inside = screenshot_path(dir.to_str(), "ABCDEF12-3456").unwrap();
        assert_eq!(inside.parent(), Some(dir.as_path()));
        let name = inside.file_name().unwrap().to_string_lossy().into_owned();
        assert!(
            name.starts_with("ABCDEF12-") && name.ends_with(".png"),
            "{name}"
        );

        let fresh = format!("{}/new/", dir.display());
        let made = screenshot_path(Some(&fresh), "ABCDEF12").unwrap();
        assert_eq!(made.parent(), Some(dir.join("new").as_path()));

        assert!(matches!(
            screenshot_path(Some(""), "ABCDEF12"),
            Err(CliError::Usage(_))
        ));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn png_size_reads_the_header() {
        let mut png = vec![0u8; 24];
        png[16..20].copy_from_slice(&603u32.to_be_bytes());
        png[20..24].copy_from_slice(&1311u32.to_be_bytes());
        assert_eq!(png_size(&png), (603, 1311));
        assert_eq!(png_size(&[]), (0, 0));
    }

    #[test]
    fn flags_and_words_split() {
        let args = parse_args(
            &[
                "tap".into(),
                "e4".into(),
                "--expect".into(),
                "Log in".into(),
                "--json".into(),
            ],
            &["--expect"],
        )
        .unwrap();
        assert_eq!(args.words, ["tap", "e4"]);
        assert_eq!(args.flag("--expect"), Some("Log in"));
        assert!(args.bool("--json"));
    }

    #[test]
    fn unknown_flag_refused() {
        assert!(parse_args(&["--nope".into()], &["--udid"]).is_err());
    }

    #[test]
    fn value_flag_must_apply() {
        assert!(parse_args(&["devices".into(), "--expect".into(), "x".into()], &[]).is_err());
    }

    fn test_recording(byte: u8, frames: Vec<simdriver_agent::RecordedFrame>) -> Recording {
        Recording {
            ended: Ended::Idle,
            extended: false,
            requested_ms: 5000,
            duration_ms: 900,
            action_start_ms: Some(100),
            action_end_ms: Some(200),
            settled_ms: Some(600),
            frames,
            health: simdriver_agent::RecordHealth {
                received: 10,
                sampled: 5,
                skipped_by_pipeline: 0,
                skipped_by_recorder: 0,
                max_gap_ms: 17.0,
                gaps_over_34ms: 0,
            },
            error: None,
            sheets: vec![vec![byte]],
            sheet_files: Vec::new(),
        }
    }

    fn strings(items: &[&str]) -> Vec<String> {
        items.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn durations_parse() {
        assert_eq!(parse_duration("5s").unwrap(), Duration::from_secs(5));
        assert_eq!(parse_duration("1.5s").unwrap(), Duration::from_millis(1500));
        assert_eq!(parse_duration("800ms").unwrap(), Duration::from_millis(800));
        assert_eq!(parse_duration("250").unwrap(), Duration::from_millis(250));
        for bad in ["5m", "abc", "", "s", "-1s", "1.2.3s"] {
            assert!(
                matches!(parse_duration(bad), Err(CliError::Usage(_))),
                "{bad}"
            );
        }
    }

    #[test]
    fn record_args_split_at_double_dash() {
        let all = strings(&["--time", "2s", "--full", "--", "tap", "Log in"]);
        let (head, tail) = split_double_dash(&all);
        let args = parse_args(head, &["--time", "--full"]).unwrap();
        let opts = record_options(&args).unwrap();
        assert_eq!(opts.time, Duration::from_secs(2));
        assert!(opts.full);
        assert_eq!(
            parse_record_tail(tail).unwrap(),
            vec![Step::Tap {
                target: "Log in".into()
            }]
        );
        let none = strings(&["--fps", "25"]);
        let (head, tail) = split_double_dash(&none);
        assert_eq!(head.len(), 2);
        assert!(parse_record_tail(tail).unwrap().is_empty());
        let steps = strings(&["steps", r#"[{"action":"tap","target":"a"}]"#]);
        assert_eq!(parse_record_tail(&steps).unwrap().len(), 1);
    }

    #[test]
    fn expect_under_record_is_usage() {
        let tail = strings(&["tap", "Log in", "--expect", "Welcome"]);
        assert!(matches!(parse_record_tail(&tail), Err(CliError::Usage(_))));
    }

    #[test]
    fn frame_file_names() {
        assert_eq!(frame_file_name(3, 250), "frame-03-250ms.png");
        assert_eq!(frame_file_name(12, 4000), "frame-12-4000ms.png");
    }

    #[test]
    fn out_dir_is_created_and_reused() {
        use simdriver_agent::RecordedFrame;
        let dir = std::env::temp_dir().join(format!(
            "simdriver-cli-test-{}-{}",
            std::process::id(),
            unix_ms()
        ));
        let out = dir.join("nested").join("rec");
        let build = |byte: u8| {
            test_recording(
                byte,
                vec![RecordedFrame {
                    index: 1,
                    t_ms: 250,
                    change: 0.5,
                    png: vec![byte],
                    file: None,
                }],
            )
        };
        let mut first = build(1);
        write_recording(&out, &mut first).unwrap();
        std::fs::write(out.join("keep.txt"), "mine").unwrap();
        let mut second = build(2);
        write_recording(&out, &mut second).unwrap();
        assert_eq!(std::fs::read(out.join("sheet-1.png")).unwrap(), [2]);
        assert_eq!(std::fs::read(out.join("frame-01-250ms.png")).unwrap(), [2]);
        assert_eq!(
            std::fs::read_to_string(out.join("keep.txt")).unwrap(),
            "mine"
        );
        let json: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(out.join("record.json")).unwrap())
                .unwrap();
        let sheet = json["sheetFiles"][0].as_str().unwrap();
        assert!(std::path::Path::new(sheet).is_absolute());
        assert!(sheet.ends_with("sheet-1.png"));
        assert!(
            json["frames"][0]["file"]
                .as_str()
                .unwrap()
                .ends_with("frame-01-250ms.png")
        );
        assert_eq!(second.sheet_files, [sheet.to_string()]);
        let _ = std::fs::remove_dir_all(&dir);
    }

    fn scratch(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "simdriver-cli-test-{name}-{}-{}",
            std::process::id(),
            unix_ms()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn one_frame(byte: u8) -> Recording {
        test_recording(
            byte,
            vec![simdriver_agent::RecordedFrame {
                index: 0,
                t_ms: 0,
                change: 0.0,
                png: vec![byte],
                file: None,
            }],
        )
    }

    #[test]
    fn unwritable_out_is_refused_before_the_driver_starts() {
        // `prepare_record_dir` runs before `start_driver`; it touches only the file system.
        let dir = scratch("prepare");
        let file = dir.join("a-file");
        std::fs::write(&file, "x").unwrap();
        for bad in [file.clone(), file.join("below")] {
            match prepare_record_dir(Some(&bad), "ABCDEF12-0000") {
                Err(CliError::Fail(m)) => assert!(m.contains("cannot write the recording"), "{m}"),
                other => panic!("{bad:?}: {other:?}"),
            }
        }
        assert!(matches!(
            prepare_record_dir(Some(std::path::Path::new("")), "x"),
            Err(CliError::Usage(_))
        ));
        // A directory without write permission (root could write anyway: skip then).
        let locked = dir.join("locked");
        std::fs::create_dir(&locked).unwrap();
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&locked, std::fs::Permissions::from_mode(0o555)).unwrap();
        if std::fs::write(locked.join("probe"), "").is_err() {
            assert!(prepare_record_dir(Some(&locked), "x").is_err());
            assert!(prepare_record_dir(Some(&locked.join("sub")), "x").is_err());
        }
        std::fs::set_permissions(&locked, std::fs::Permissions::from_mode(0o755)).unwrap();
        // A good one is created, absolute, empty (the probe is gone), and marked created.
        let fresh = dir.join("nested").join("rec");
        let ready = prepare_record_dir(Some(&fresh), "x").unwrap();
        assert!(ready.path.is_absolute() && ready.path.is_dir() && ready.created);
        assert_eq!(std::fs::read_dir(&ready.path).unwrap().count(), 0);
        // An existing one is reused, not marked created.
        assert!(!prepare_record_dir(Some(&fresh), "x").unwrap().created);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_failed_write_falls_back_to_a_temp_dir() {
        let dir = scratch("fallback");
        // The target became a file after it was prepared: writing there fails.
        let target = dir.join("target");
        std::fs::write(&target, "x").unwrap();
        let fallback = dir.join("fallback");
        let mut rec = one_frame(7);
        let error = save_recording(&target, &fallback, &mut rec).expect("an error");
        assert!(error.contains(&target.display().to_string()), "{error}");
        assert!(
            error.contains(&format!("wrote it to {} instead", fallback.display())),
            "{error}"
        );
        assert_eq!(std::fs::read(fallback.join("sheet-1.png")).unwrap(), [7]);
        assert_eq!(
            std::fs::read(fallback.join("frame-00-0ms.png")).unwrap(),
            [7]
        );
        assert!(fallback.join("record.json").is_file());
        assert!(rec.sheet_files[0].starts_with(&*fallback.to_string_lossy()));
        assert!(
            rec.frames[0]
                .file
                .as_deref()
                .is_some_and(|f| f.starts_with(&*fallback.to_string_lossy()))
        );

        // A good target takes it: no error, nothing in the fallback.
        let good = dir.join("good");
        let unused = dir.join("unused");
        assert_eq!(save_recording(&good, &unused, &mut one_frame(1)), None);
        assert!(good.join("record.json").is_file() && !unused.exists());

        // Both fail: the error says so and no path is left pointing at a missing file.
        let mut rec = one_frame(3);
        let error = save_recording(&target, &target.join("sub"), &mut rec).expect("an error");
        assert!(error.contains("nor to"), "{error}");
        assert!(rec.sheet_files.is_empty() && rec.frames[0].file.is_none());
        let _ = std::fs::remove_dir_all(&dir);
    }
}

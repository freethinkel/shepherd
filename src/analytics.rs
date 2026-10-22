//! Aptabase: how many people use Shepherd each day, on which OS, with which agents. Anonymous: an `active` event a day
//! (at launch, then with the first snapshot of each new UTC day) and an `agent` event a day for each agent herdr
//! names in the panes (`name`: `claude`, `codex`…). Nothing else leaves the machine. `DO_NOT_TRACK=1` turns it off.
use std::collections::HashSet;
use std::sync::Mutex;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use serde_json::{Value, json};

const APP_KEY: &str = "A-SH-5120995280";
const URL: &str = "https://aptabase.freethinkel.dev/api/v0/events";

/// The UTC day the events went out for, its session id, and the agents already reported that day.
static SENT: Mutex<Option<(u64, String, HashSet<String>)>> = Mutex::new(None);

/// Reports the day and `agents` the first time each comes up today; cheap to call on every snapshot.
pub fn seen<'a>(agents: impl IntoIterator<Item = &'a str>) {
    if !enabled() {
        return;
    }
    let secs = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs();
    let mut events = Vec::new();
    let mut sent = SENT.lock().unwrap_or_else(|e| e.into_inner());
    if sent.as_ref().is_none_or(|(day, _, _)| *day != secs / 86_400) {
        // a session a day, as Aptabase's SDKs start a new one after hours of quiet
        let session = (secs * 100_000_000 + u64::from(SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().subsec_nanos()) % 100_000_000).to_string();
        *sent = Some((secs / 86_400, session, HashSet::new()));
        events.push(("active", None));
    }
    let (_, session, agents_sent) = sent.as_mut().expect("set above");
    for agent in agents {
        if agents_sent.insert(agent.to_string()) {
            events.push(("agent", Some(json!({"name": agent}))));
        }
    }
    if events.is_empty() {
        return;
    }
    let (session, at) = (session.clone(), timestamp(secs));
    drop(sent);
    std::thread::spawn(move || {
        let os = os(); // off the main thread: `sw_vers` is a process
        let body: Vec<Value> = events.into_iter().map(|(name, props)| event(name, props, &session, &at, &os)).collect();
        let agent: ureq::Agent = ureq::Agent::config_builder().timeout_global(Some(Duration::from_secs(15))).build().into();
        // ponytail: an event that fails to send is dropped, no queue or retry; the next day's goes out anyway
        let _ = agent.post(URL).header("App-Key", APP_KEY).content_type("application/json").send(Value::Array(body).to_string());
    });
}

fn enabled() -> bool {
    std::env::var("DO_NOT_TRACK").map_or(true, |v| v.is_empty() || v == "0") && crate::mock::dir().is_none()
}

fn event(name: &str, props: Option<Value>, session: &str, at: &str, (os_name, os_version): &(String, String)) -> Value {
    json!({
        "timestamp": at,
        "sessionId": session,
        "eventName": name,
        "systemProps": {
            "isDebug": cfg!(debug_assertions),
            "osName": os_name,
            "osVersion": os_version,
            "locale": "",
            "appVersion": env!("CARGO_PKG_VERSION"),
            "sdkVersion": concat!("shepherd@", env!("CARGO_PKG_VERSION")),
        },
        "props": props,
    })
}

/// The OS's name and version, as Aptabase's SDKs name them (macOS, Windows, the Linux distribution).
fn os() -> (String, String) {
    if cfg!(target_os = "macos") {
        let out = crate::process::command("sw_vers").arg("-productVersion").output().ok();
        let version = out.map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string()).unwrap_or_default();
        return ("macOS".into(), version);
    }
    if cfg!(target_os = "linux") {
        let release = std::fs::read_to_string("/etc/os-release").unwrap_or_default();
        let field = |key: &str| {
            release.lines().find_map(|l| l.strip_prefix(key)?.strip_prefix('=')).unwrap_or_default().trim_matches('"').to_string()
        };
        let name = field("NAME");
        return (if name.is_empty() { "Linux".into() } else { name }, field("VERSION_ID"));
    }
    // ponytail: no Windows version; read it from the registry if the split by Windows release ever matters
    ("Windows".into(), String::new())
}

/// `secs` since the epoch as RFC 3339 in UTC.
fn timestamp(secs: u64) -> String {
    // Howard Hinnant's days-to-civil
    let (days, rem) = ((secs / 86_400) as i64, secs % 86_400);
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = yoe + era * 400 + i64::from(month <= 2);
    format!("{year:04}-{month:02}-{day:02}T{:02}:{:02}:{:02}Z", rem / 3600, rem % 3600 / 60, rem % 60)
}

#[cfg(test)]
mod tests {
    use super::timestamp;

    #[test]
    fn timestamps_are_utc_rfc3339() {
        assert_eq!(timestamp(0), "1970-01-01T00:00:00Z");
        assert_eq!(timestamp(951_782_400), "2000-02-29T00:00:00Z");
        assert_eq!(timestamp(1_792_188_245), "2026-10-16T22:04:05Z");
    }
}

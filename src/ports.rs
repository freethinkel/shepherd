//! The TCP ports this Mac's dev servers listen on, from `lsof`, for the right panel's Ports tab. A process whose
//! directory is `/` or home (an app opened from Finder, a system daemon) is left out: dev servers run from a project.
use std::collections::HashMap;

#[derive(Clone, Debug, PartialEq)]
pub struct Listener {
    pub port: u16,
    pub pid: u32,
    /// The program, as `lsof` names it (cut to 15 characters on macOS).
    pub name: String,
    /// Its working directory.
    pub cwd: String,
}

impl Listener {
    pub fn url(&self) -> String {
        format!("http://localhost:{}", self.port)
    }
}

/// `lsof -F pcn` of listening sockets: (pid, program, port), one per port of a process (its IPv4 and IPv6 sockets
/// are one).
fn parse_listening(out: &str) -> Vec<(u32, String, u16)> {
    let (mut pid, mut name, mut found) = (0, String::new(), Vec::new());
    for line in out.lines() {
        let (tag, value) = line.split_at(line.len().min(1));
        match tag {
            "p" => pid = value.parse().unwrap_or(0),
            "c" => name = value.to_string(),
            "n" => {
                let port = value.rsplit(':').next().and_then(|p| p.parse().ok());
                if let Some(port) = port.filter(|_| pid != 0) {
                    let entry = (pid, name.clone(), port);
                    if !found.contains(&entry) {
                        found.push(entry);
                    }
                }
            }
            _ => {}
        }
    }
    found
}

/// `lsof -F pn` of `-d cwd`: each process's directory.
fn parse_cwds(out: &str) -> HashMap<u32, String> {
    let (mut pid, mut cwds) = (0, HashMap::new());
    for line in out.lines() {
        match line.split_at(line.len().min(1)) {
            ("p", v) => pid = v.parse().unwrap_or(0),
            ("n", v) => {
                cwds.insert(pid, v.to_string());
            }
            _ => {}
        }
    }
    cwds
}

fn lsof(args: &[&str]) -> Option<String> {
    let out = crate::process::command(crate::ui::statusbar::tool("lsof")).args(args).output().ok()?;
    // lsof exits 1 when one of the processes asked about is gone or not ours; what it found is still printed
    Some(String::from_utf8_lossy(&out.stdout).into_owned())
}

/// The ports listened on, by port. `None` when `lsof` can't run.
pub fn read(_: &str) -> Option<Vec<Listener>> {
    let listening = parse_listening(&lsof(&["-nP", "-iTCP", "-sTCP:LISTEN", "-Fpcn"])?);
    let mut pids: Vec<String> = listening.iter().map(|(pid, ..)| pid.to_string()).collect();
    pids.dedup();
    let cwds = if pids.is_empty() { HashMap::new() } else { parse_cwds(&lsof(&["-a", "-d", "cwd", "-Fpn", "-p", &pids.join(",")]).unwrap_or_default()) };
    let home = std::env::home_dir().map(|h| h.to_string_lossy().into_owned());
    let mut found: Vec<Listener> = listening
        .into_iter()
        .filter_map(|(pid, name, port)| {
            let cwd = cwds.get(&pid).filter(|d| d.as_str() != "/" && Some(d.as_str()) != home.as_deref())?.clone();
            Some(Listener { port, pid, name, cwd })
        })
        .collect();
    found.sort_by_key(|l| l.port);
    Some(found)
}

/// Whether `cwd` is `dir` or under it.
pub fn under(cwd: &str, dir: &str) -> bool {
    let dir = dir.trim_end_matches('/');
    cwd == dir || cwd.strip_prefix(dir).is_some_and(|rest| rest.starts_with('/'))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_process_listening_twice_on_one_port_is_one_row() {
        let out = "p6118\ncnode\nf32\nn*:3100\nf33\nn[::1]:3100\nf34\nn127.0.0.1:9229\np793\ncrapportd\nf11\nn*:49156\n";
        assert_eq!(
            parse_listening(out),
            vec![(6118, "node".into(), 3100), (6118, "node".into(), 9229), (793, "rapportd".into(), 49156)]
        );
    }

    #[test]
    fn cwds_by_process() {
        let cwds = parse_cwds("p6118\nfcwd\nn/Users/me/app\np793\nfcwd\nn/\n");
        assert_eq!((cwds[&6118].as_str(), cwds[&793].as_str()), ("/Users/me/app", "/"));
    }

    #[test]
    fn under_a_directory_not_beside_it() {
        assert!(under("/a/app", "/a/app/") && under("/a/app/web", "/a/app"));
        assert!(!under("/a/app-2", "/a/app"));
    }
}

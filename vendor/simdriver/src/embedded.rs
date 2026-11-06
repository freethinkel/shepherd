//! The helper binaries, compiled into this crate and unpacked on first use. The build's own
//! output directory can't be relied on: `cargo install` deletes it after installing.
use std::collections::hash_map::DefaultHasher;
use std::ffi::OsString;
use std::fs;
use std::hash::{Hash, Hasher};
use std::io::{self, Write};
use std::os::unix::fs::OpenOptionsExt;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

const HELPERS: [(&str, &[u8]); 4] = [
    (
        "sdframe",
        include_bytes!(concat!(env!("OUT_DIR"), "/helpers/sdframe")),
    ),
    (
        "sdhid",
        include_bytes!(concat!(env!("OUT_DIR"), "/helpers/sdhid")),
    ),
    (
        "sdax",
        include_bytes!(concat!(env!("OUT_DIR"), "/helpers/sdax")),
    ),
    // the simulator's, not the Mac's: it runs inside it
    (
        "sdaxsettings",
        include_bytes!(concat!(env!("OUT_DIR"), "/helpers/sdaxsettings")),
    ),
];

/// Where this build's helpers are unpacked, or why they couldn't be. Only ever the user's own
/// cache: helpers run from a shared directory (like `/tmp`) could be swapped by another user.
pub(crate) fn dir() -> &'static Result<PathBuf, String> {
    static DIR: OnceLock<Result<PathBuf, String>> = OnceLock::new();
    DIR.get_or_init(|| {
        let root = cache_root(std::env::var_os("XDG_CACHE_HOME"), std::env::var_os("HOME"))
            .ok_or("no cache directory (HOME is not set)")?
            .join("simdriver/helpers");
        unpack(&root).map_err(|e| format!("could not unpack into {}: {e}", root.display()))
    })
}

/// `$XDG_CACHE_HOME`, else `$HOME/.cache`. A relative path is ignored: it would resolve
/// against the working directory, which may not be the user's.
fn cache_root(xdg: Option<OsString>, home: Option<OsString>) -> Option<PathBuf> {
    let xdg = xdg.map(PathBuf::from);
    let home = home.map(|home| PathBuf::from(home).join(".cache"));
    xdg.filter(|p| p.is_absolute())
        .or(home.filter(|p| p.is_absolute()))
}

/// One directory per build, so processes from different versions never share helpers.
fn build_id() -> String {
    let mut hasher = DefaultHasher::new();
    HELPERS.hash(&mut hasher);
    format!("{:016x}", hasher.finish())
}

fn unpack(root: &Path) -> io::Result<PathBuf> {
    let dir = root.join(build_id());
    fs::create_dir_all(&dir)?;
    for (name, bytes) in HELPERS {
        let path = dir.join(name);
        if fs::read(&path).is_ok_and(|on_disk| on_disk == bytes) {
            continue;
        }
        // Write a new file then rename: a helper that's running is never overwritten in
        // place, and another process never sees a partial binary.
        let tmp = dir.join(format!(".{name}.{}.tmp", std::process::id()));
        let mut file = fs::OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .mode(0o755)
            .open(&tmp)?;
        file.write_all(bytes)?;
        drop(file);
        fs::rename(&tmp, &path)?;
    }
    Ok(dir)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::process::Command;

    fn scratch(tag: &str) -> PathBuf {
        let root =
            std::env::temp_dir().join(format!("simdriver-embed-{}-{tag}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        root
    }

    #[test]
    fn unpacked_helpers_run() {
        let root = scratch("run");
        let dir = unpack(&root).unwrap();
        for (name, _) in HELPERS {
            let out = Command::new(dir.join(name)).output().unwrap();
            assert_eq!(
                out.status.code(),
                Some(2),
                "{name} with no args prints usage"
            );
        }
        assert_eq!(
            fs::read_dir(&dir).unwrap().count(),
            HELPERS.len(),
            "no temp files left behind"
        );
        fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn a_damaged_helper_is_replaced() {
        let root = scratch("damaged");
        let dir = unpack(&root).unwrap();
        fs::write(dir.join("sdhid"), b"truncated").unwrap();
        unpack(&root).unwrap();
        let (_, bytes) = HELPERS[1];
        assert_eq!(fs::read(dir.join("sdhid")).unwrap(), bytes);
        fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn cache_root_precedence() {
        let root = |xdg: Option<&str>, home: Option<&str>| {
            cache_root(xdg.map(Into::into), home.map(Into::into))
        };
        assert_eq!(root(Some("/x"), Some("/h")), Some("/x".into()));
        assert_eq!(root(None, Some("/h")), Some("/h/.cache".into()));
        assert_eq!(root(None, None), None);
        assert_eq!(root(Some(""), Some("/h")), Some("/h/.cache".into()));
        assert_eq!(root(Some("rel"), Some("/h")), Some("/h/.cache".into()));
        assert_eq!(root(None, Some("rel")), None);
    }

    #[test]
    fn an_unwritable_root_is_an_error() {
        assert!(unpack(Path::new("/nonexistent/simdriver")).is_err());
    }
}

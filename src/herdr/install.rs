//! Getting herdr when it is nowhere to be found: this system's binary from herdr.dev's release manifest (the one its
//! install.sh and `herdr update` read), checked against the manifest's SHA-256, into `~/.local/bin` where install.sh
//! puts it and Shepherd looks. macOS and Linux only: herdr ships Windows as a zip with an installer of its own.
use anyhow::{Context as _, bail};
use sha2::{Digest, Sha256};
use std::path::PathBuf;
use std::time::Duration;

const MANIFEST: &str = "https://herdr.dev/latest.json";

/// No herdr on PATH or where its installers put it, none set in settings, and one to download for this system.
pub fn missing() -> bool {
    super::session::binary() == "herdr" && target().is_some()
}

/// Downloads herdr's latest release; where it now is.
pub fn install() -> anyhow::Result<PathBuf> {
    let target = target().context("herdr has no release binary for this system")?;
    let agent: ureq::Agent = ureq::Agent::config_builder()
        .timeout_connect(Some(Duration::from_secs(15)))
        .timeout_global(Some(Duration::from_secs(300)))
        .build()
        .into();
    let manifest: serde_json::Value =
        serde_json::from_str(&agent.get(MANIFEST).call().context("cannot reach herdr.dev")?.body_mut().read_to_string()?)?;
    let (Some(url), Some(sha256)) = (manifest["assets"][&target].as_str(), manifest["sha256"][&target].as_str()) else {
        bail!("herdr's latest release has no {target} binary");
    };
    let bytes = agent
        .get(url)
        .call()
        .with_context(|| format!("cannot download {url}"))?
        .body_mut()
        .with_config()
        .limit(256 << 20)
        .read_to_vec()
        .context("the download broke off")?;
    check(&bytes, sha256)?;
    let path = crate::settings::home().join(".local/bin/herdr");
    std::fs::create_dir_all(path.parent().unwrap())?;
    // written aside and renamed in, so a half-written herdr is never on the path
    let part = path.with_extension("download");
    std::fs::write(&part, &bytes).with_context(|| part.display().to_string())?;
    #[cfg(unix)]
    std::fs::set_permissions(&part, std::os::unix::fs::PermissionsExt::from_mode(0o755))?;
    std::fs::rename(&part, &path).with_context(|| path.display().to_string())?;
    Ok(path)
}

/// The manifest's name for this system, as install.sh builds it.
fn target() -> Option<String> {
    let os = match std::env::consts::OS {
        "macos" | "linux" => std::env::consts::OS,
        _ => return None,
    };
    let arch = match std::env::consts::ARCH {
        "aarch64" | "x86_64" => std::env::consts::ARCH,
        _ => return None,
    };
    Some(format!("{os}-{arch}"))
}

fn check(bytes: &[u8], sha256: &str) -> anyhow::Result<()> {
    if format!("{:x}", Sha256::digest(bytes)) != sha256.to_ascii_lowercase() {
        bail!("the downloaded herdr's checksum does not match herdr.dev's");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn checks_the_sha256() {
        let abc = "BA7816BF8F01CFEA414140DE5DAE2223B00361A396177A9CB410FF61F20015AD";
        assert!(check(b"abc", abc).is_ok());
        assert!(check(b"abd", abc).is_err());
    }
}

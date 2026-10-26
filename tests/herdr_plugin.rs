//! The worktree hooks plugin's `event.sh`, run by its own test script under `sh` and, when installed, `dash`.
#[cfg(unix)]
#[test]
fn event_sh() {
    for shell in ["sh", "dash"] {
        if std::process::Command::new(shell).arg("-c").arg(":").status().is_err() {
            continue;
        }
        let out = std::process::Command::new(shell).arg("assets/herdr-plugin/test.sh").env("SH", shell).output().unwrap();
        assert!(out.status.success(), "{shell}:\n{}{}", String::from_utf8_lossy(&out.stdout), String::from_utf8_lossy(&out.stderr));
    }
}

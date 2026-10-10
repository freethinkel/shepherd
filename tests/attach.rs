use std::io::Write;
use std::os::unix::fs::PermissionsExt;
use std::process::{Command, Stdio};

#[test]
fn shim_bridges_frames_input_and_exit() {
    let dir = tempfile::tempdir().unwrap();
    let fake = dir.path().join("herdr");
    std::fs::write(
        &fake,
        r#"#!/bin/sh
echo "$@" > "$REC/args"
printf '%s\n' '{"type":"terminal.frame","seq":1,"encoding":"ansi","width":80,"height":24,"full":true,"bytes":"aGVsbG8="}'
cat > "$REC/stdin"
printf '%s\n' '{"type":"terminal.closed","reason":null}'
"#,
    )
    .unwrap();
    std::fs::set_permissions(&fake, std::fs::Permissions::from_mode(0o755)).unwrap();

    let mut child = Command::new(env!("CARGO_BIN_EXE_shepherd"))
        .args(["attach", "w1:p1"])
        .env("SHEPHERD_HERDR", &fake)
        .env("REC", dir.path())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    child.stdin.take().unwrap().write_all(b"ls\r").unwrap(); // dropped here: EOF
    let out = child.wait_with_output().unwrap();

    assert!(out.status.success());
    assert_eq!(out.stdout, b"hello");
    let args = std::fs::read_to_string(dir.path().join("args")).unwrap();
    assert_eq!(args.trim(), "terminal session control w1:p1 --cols 80 --rows 24");
    let input = std::fs::read_to_string(dir.path().join("stdin")).unwrap();
    assert_eq!(input, "{\"type\":\"terminal.input\",\"bytes\":\"bHMN\"}\n");
}

#[test]
fn command_line_quotes_paths() {
    let line = shepherd::attach::command_line(std::path::Path::new("/Apps/My App/shepherd"), "w1:p1");
    assert_eq!(line, "'/Apps/My App/shepherd' attach 'w1:p1'");
}

#[test]
fn shim_sends_the_pty_size_once_after_start() {
    // A Ghostty surface sizes its PTY just after spawning the shim; that first size must reach herdr
    // even if its SIGWINCH came before the shim was listening.
    let dir = tempfile::tempdir().unwrap();
    let fake = dir.path().join("herdr");
    std::fs::write(
        &fake,
        "#!/bin/sh\necho \"$@\" > \"$REC/args\"\ncat > \"$REC/stdin\"\nprintf '%s\\n' '{\"type\":\"terminal.closed\",\"reason\":null}'\n",
    )
    .unwrap();
    std::fs::set_permissions(&fake, std::fs::Permissions::from_mode(0o755)).unwrap();

    let (mut master, mut slave) = (0, 0);
    let mut ws = libc::winsize { ws_row: 30, ws_col: 100, ws_xpixel: 0, ws_ypixel: 0 };
    assert_eq!(unsafe { libc::openpty(&mut master, &mut slave, std::ptr::null_mut(), std::ptr::null_mut(), &mut ws) }, 0);
    unsafe { libc::fcntl(master, libc::F_SETFD, libc::FD_CLOEXEC) }; // or the shim inherits it and never sees a hangup
    let tty = unsafe { <std::fs::File as std::os::fd::FromRawFd>::from_raw_fd(slave) };

    let mut child = Command::new(env!("CARGO_BIN_EXE_shepherd"))
        .args(["attach", "w1:p1"])
        .env("SHEPHERD_HERDR", &fake)
        .env("REC", dir.path())
        .stdin(Stdio::from(tty))
        .stdout(Stdio::null())
        .spawn()
        .unwrap();
    std::thread::sleep(std::time::Duration::from_millis(400));
    unsafe { libc::close(master) }; // hang up: the shim's stdin read fails and it ends the session
    let status = child.wait().unwrap();

    assert!(status.success());
    let args = std::fs::read_to_string(dir.path().join("args")).unwrap();
    assert_eq!(args.trim(), "terminal session control w1:p1 --cols 100 --rows 30");
    let input = std::fs::read_to_string(dir.path().join("stdin")).unwrap();
    assert!(input.contains("\"type\":\"terminal.resize\""), "no resize sent: {input:?}");
    assert!(input.contains("\"cols\":100") && input.contains("\"rows\":30"), "{input:?}");
}

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

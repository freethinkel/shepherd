//! The helper programs Shepherd runs (herdr, git, gh, ssh). On Windows a console program started by a windowed app
//! opens a console window of its own: these start without one.
use std::ffi::OsStr;
use std::process::Command;

#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

pub fn command(program: impl AsRef<OsStr>) -> Command {
    #[allow(unused_mut)]
    let mut command = Command::new(program);
    #[cfg(windows)]
    std::os::windows::process::CommandExt::creation_flags(&mut command, CREATE_NO_WINDOW);
    command
}

/// `line` run by the shell: `sh -c`, or on Windows `cmd /d /c` as herdr runs it there.
pub fn shell(line: &str) -> Command {
    #[cfg(windows)]
    {
        let mut cmd = command(std::env::var_os("ComSpec").unwrap_or_else(|| "cmd.exe".into()));
        // cmd parses its own command line: the line goes as written, not quoted as one argument
        std::os::windows::process::CommandExt::raw_arg(cmd.args(["/d", "/c"]), line);
        cmd
    }
    #[cfg(not(windows))]
    {
        let mut sh = command("sh");
        sh.args(["-c", line]);
        sh
    }
}


//! Explicit user action only. No shell interpolation and no guest/PTY ownership.
use std::{
    path::{Path, PathBuf},
    process::Command,
};

fn find(program: &str) -> Option<PathBuf> {
    std::env::split_paths(&std::env::var_os("PATH")?)
        .map(|dir| dir.join(program))
        .find(|path| {
            let Ok(meta) = path.metadata() else {
                return false;
            };
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                meta.is_file() && meta.permissions().mode() & 0o111 != 0
            }
            #[cfg(not(unix))]
            {
                meta.is_file()
            }
        })
}

fn command(terminal: &Path, separator: &str, minicom: &Path, endpoint: &str) -> Command {
    let mut command = Command::new(terminal);
    command
        .arg(separator)
        .arg(minicom)
        .args(["-o", "-8", "-b", "9600", "-D", endpoint]);
    command
}

pub fn open(endpoint: &str) -> Result<(), String> {
    let minicom = find("minicom").ok_or("minicom is not on PATH. Install it, or connect from your own terminal using the command shown in the control window.")?;
    for (program, separator) in [
        ("x-terminal-emulator", "-e"),
        ("gnome-terminal", "--"),
        ("konsole", "-e"),
        ("xfce4-terminal", "-x"),
        ("xterm", "-e"),
    ] {
        if let Some(terminal) = find(program) {
            let mut child = command(&terminal, separator, &minicom, endpoint)
                .spawn()
                .map_err(|e| format!("Opening {program}: {e}"))?;
            // Reap without blocking SDL. Closing the terminal never closes the guest.
            std::thread::spawn(move || {
                let _ = child.wait();
            });
            return Ok(());
        }
    }
    Err(
        "No supported terminal found on PATH; run the displayed minicom command in your terminal."
            .into(),
    )
}

pub fn recipe(endpoint: &str) -> String {
    format!(
        "minicom -o -8 -b 9600 -D '{}'",
        endpoint.replace('\'', "'\\''")
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn endpoint_is_one_literal_argument_without_a_shell() {
        let endpoint = "/tmp/serial ' $(touch BAD) ; a";
        let cmd = command(Path::new("xterm"), "-e", Path::new("minicom"), endpoint);
        assert_eq!(
            cmd.get_args().collect::<Vec<_>>(),
            ["-e", "minicom", "-o", "-8", "-b", "9600", "-D", endpoint]
        );
        assert_eq!(
            recipe("/tmp/a'b"),
            "minicom -o -8 -b 9600 -D '/tmp/a'\\''b'"
        );
    }
}

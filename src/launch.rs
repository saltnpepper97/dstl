use std::process::{Command, Stdio};
use std::os::unix::process::CommandExt;
use crate::app::AppEntry;
use crate::config::LauncherConfig;

pub fn launch_app(entry: &AppEntry, config: &LauncherConfig) {
    let terminal = &config.terminal;

    let mut cmd = if entry.terminal || entry.needs_terminal() {
        // Terminal app
        let mut c = Command::new(terminal);
        c.arg("-e").arg(&entry.exec);
        c
    } else {
        // GUI app
        let mut c = Command::new("sh");
        c.arg("-c").arg(&entry.exec);
        c
    };

    // Fully detach (don't block, don't get killed with parent)
    unsafe {
        cmd.pre_exec(|| {
            libc::setsid();
            Ok(())
        });
    }

    let _ = cmd
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn();
}

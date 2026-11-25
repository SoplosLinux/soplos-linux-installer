// The installer updates itself from the live session: an ISO already
// published keeps the installer it was built with, so at start the window
// asks the repository for a newer one, offers it, installs it and opens again.
// Only in the live session (the one with the image of the system) and only
// when sudo works without a password, as the live user's does. The installed
// system is copied from the squashfs, so the update stays in the live session.

use std::os::unix::process::CommandExt;
use std::path::Path;
use std::process::{Command, Stdio};
use tauri::AppHandle;

const PACKAGE: &str = "soplos-linux-installer";

/// Where Debian's live systems keep the image of the system
/// (helper/system.rs).
const SQUASHFS: [&str; 3] = [
    "/run/live/medium/live/filesystem.squashfs",
    "/run/initramfs/live/live/filesystem.squashfs",
    "/lib/live/mount/medium/live/filesystem.squashfs",
];

fn live_session() -> bool {
    SQUASHFS.iter().any(|p| Path::new(p).exists())
}

fn quiet(program: &str, args: &[&str]) -> Command {
    let mut c = Command::new(program);
    c.args(args)
        .env("LC_ALL", "C")
        .env("DEBIAN_FRONTEND", "noninteractive")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null());
    c
}

fn sudo_works() -> bool {
    quiet("sudo", &["-n", "true"]).status().is_ok_and(|s| s.success())
}

/// "Installed:" and "Candidate:" of `apt-cache policy`.
fn policy(text: &str) -> (String, String) {
    let field = |key: &str| {
        text.lines()
            .find_map(|l| l.trim().strip_prefix(key))
            .map(|v| v.trim().to_string())
            .unwrap_or_default()
    };
    (field("Installed:"), field("Candidate:"))
}

fn newer(candidate: &str, installed: &str) -> bool {
    quiet("dpkg", &["--compare-versions", candidate, "gt", installed])
        .status()
        .is_ok_and(|s| s.success())
}

/// The version of the installer the repository has, when it is newer than
/// the one running. None without network, outside the live session or when
/// it is already the latest.
#[tauri::command(async)]
pub fn check_update() -> Option<String> {
    if !live_session() || !sudo_works() {
        return None;
    }
    let updated = quiet("sudo", &["-n", "apt-get", "update", "-q"]).status().is_ok_and(|s| s.success());
    if !updated {
        return None;
    }
    let output = quiet("apt-cache", &["policy", PACKAGE]).output().ok()?;
    let (installed, candidate) = policy(&String::from_utf8_lossy(&output.stdout));
    if installed.is_empty() || installed == "(none)" || candidate.is_empty() || candidate == "(none)" {
        return None;
    }
    newer(&candidate, &installed).then_some(candidate)
}

/// Installs the new installer, starts it again once this window is gone
/// (the single instance would hand a second one to this window) and closes.
#[tauri::command(async)]
pub fn apply_update(app: AppHandle) -> Result<(), String> {
    let status = quiet(
        "sudo",
        &["-n", "apt-get", "install", "-y", "-q", "-o", "Dpkg::Options::=--force-confold", PACKAGE],
    )
    .status()
    .map_err(|e| e.to_string())?;
    if !status.success() {
        return Err(format!("apt-get install {PACKAGE} failed"));
    }
    // The running binary was replaced: its path ends in " (deleted)".
    let exe = std::env::current_exe()
        .map(|p| p.to_string_lossy().trim_end_matches(" (deleted)").to_string())
        .unwrap_or_else(|_| format!("/usr/bin/{PACKAGE}"));
    let pid = std::process::id().to_string();
    Command::new("sh")
        .args(["-c", "while kill -0 \"$1\" 2>/dev/null; do sleep 0.2; done; exec \"$2\"", "sh", &pid, &exe])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .process_group(0)
        .spawn()
        .map_err(|e| e.to_string())?;
    app.exit(0);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::policy;

    #[test]
    fn reads_policy() {
        let text = "soplos-linux-installer:\n  Installed: 3.0.0\n  Candidate: 3.0.1\n  Version table:\n";
        assert_eq!(policy(text), ("3.0.0".into(), "3.0.1".into()));
    }
}

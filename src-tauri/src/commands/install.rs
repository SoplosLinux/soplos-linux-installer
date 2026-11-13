// Starts the helper and passes its progress to the window (DESIGN.md,
// section 2). The plan, passwords included, goes on the helper's stdin and is
// never written to disk.
//
// SLI_DRY_RUN=1 in the environment makes it a dry run: the helper runs as
// the user, changes nothing and lists the commands it would run.

use std::io::{BufRead, BufReader, Write};
use std::process::{Command, Stdio};
use tauri::{AppHandle, Emitter};

const EVENT: &str = "install-event";

fn sudo_works() -> bool {
    Command::new("sudo")
        .args(["-n", "true"])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .is_ok_and(|s| s.success())
}

#[tauri::command]
pub fn start_install(app: AppHandle, plan: serde_json::Value) -> Result<(), String> {
    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
    let dry_run = std::env::var("SLI_DRY_RUN").is_ok_and(|v| v == "1");
    let mut command = if dry_run {
        Command::new(&exe)
    } else if sudo_works() {
        // The live user has passwordless sudo.
        let mut c = Command::new("sudo");
        c.arg("-n").arg(&exe);
        c
    } else {
        // Outside the live session (a test machine), polkit asks.
        let mut c = Command::new("pkexec");
        c.arg(&exe);
        c
    };
    command.arg("--helper");
    if dry_run {
        command.arg("--dry-run");
    }
    let mut child = command
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| e.to_string())?;

    let text = serde_json::to_string(&plan).map_err(|e| e.to_string())?;
    if let Some(mut stdin) = child.stdin.take() {
        stdin.write_all(text.as_bytes()).map_err(|e| e.to_string())?;
    }

    let stdout = child.stdout.take().ok_or("no stdout")?;
    let stderr = child.stderr.take().ok_or("no stderr")?;
    std::thread::spawn(move || {
        let mut finished = false;
        for line in BufReader::new(stdout).lines().map_while(Result::ok) {
            match serde_json::from_str::<serde_json::Value>(&line) {
                Ok(event) => {
                    finished |= event.get("kind").and_then(|k| k.as_str()) == Some("done");
                    let _ = app.emit(EVENT, event);
                }
                Err(_) => {
                    let _ = app.emit(EVENT, serde_json::json!({ "kind": "log", "line": line }));
                }
            }
        }
        let errors: Vec<String> = BufReader::new(stderr).lines().map_while(Result::ok).collect();
        let _ = child.wait();
        // The helper did not get to say how it ended: sudo or pkexec refused,
        // or it crashed.
        if !finished {
            let message = errors.last().cloned().unwrap_or_else(|| "the helper stopped".into());
            let _ = app.emit(EVENT, serde_json::json!({ "kind": "done", "success": false, "message": message, "dryRun": dry_run }));
        }
    });
    Ok(())
}

/// Restarts the computer at the end. The live user may reboot through
/// logind; sudo is the fallback.
#[tauri::command]
pub fn reboot() -> Result<(), String> {
    let plain = Command::new("systemctl").arg("reboot").status();
    if plain.is_ok_and(|s| s.success()) {
        return Ok(());
    }
    let status = Command::new("sudo").args(["-n", "systemctl", "reboot"]).status().map_err(|e| e.to_string())?;
    if status.success() { Ok(()) } else { Err("the computer could not be restarted".into()) }
}

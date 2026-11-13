// What the installer needs to know about the live session before anything
// else: the edition being installed and the firmware of the computer. The
// edition detection is the one of Soplos Welcome Live.

use serde::Serialize;
use std::fs;
use std::path::Path;

#[derive(Serialize)]
pub struct EnvironmentInfo {
    /// "tyron" (Xfce), "tyson" (Plasma), "boro" (GNOME) or "unknown".
    pub edition: String,
    /// "uefi" or "bios": it decides which boot loaders can be offered.
    pub firmware: String,
}

fn desktop_environment() -> String {
    let current_desktop = std::env::var("XDG_CURRENT_DESKTOP").unwrap_or_default().to_lowercase();
    if current_desktop.contains("gnome") {
        "gnome"
    } else if current_desktop.contains("kde") || current_desktop.contains("plasma") {
        "kde"
    } else if current_desktop.contains("xfce") {
        "xfce"
    } else {
        "unknown"
    }
    .to_string()
}

/// The edition file wins; without it the desktop decides, as in Welcome Live.
pub fn edition() -> String {
    if let Ok(content) = fs::read_to_string("/etc/soplos-edition") {
        let content = content.trim().to_lowercase();
        for name in ["tyron", "tyson", "boro"] {
            if content.contains(name) {
                return name.to_string();
            }
        }
        return "unknown".to_string();
    }
    match desktop_environment().as_str() {
        "xfce" => "tyron",
        "kde" => "tyson",
        "gnome" => "boro",
        _ => "unknown",
    }
    .to_string()
}

/// A computer started with UEFI exposes /sys/firmware/efi.
pub fn firmware() -> String {
    if Path::new("/sys/firmware/efi").exists() { "uefi" } else { "bios" }.to_string()
}

#[tauri::command]
pub fn get_environment_info() -> EnvironmentInfo {
    EnvironmentInfo { edition: edition(), firmware: firmware() }
}

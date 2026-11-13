// The checks of the welcome step (DESIGN.md, section 3): disk, memory and
// internet stop the installation, the battery only warns. The figures come
// from the data file of the edition. Ported from Soplos Welcome Live
// (system.rs and installer.rs), adapted to compare against requirements.

use crate::commands::environment;
use crate::edition_data;
use serde::Serialize;
use std::fs;
use std::net::{SocketAddr, TcpStream};
use std::process::{Command, Stdio};
use std::time::Duration;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Requirements {
    /// The name of what is installed, from the data file ("" if unknown).
    pub edition_name: String,
    /// The largest disk Soplos can be installed on, in GB. None: no disk.
    pub disk_gb: Option<f64>,
    pub disk_required_gb: Option<f64>,
    pub memory_gb: Option<f64>,
    pub memory_required_gb: Option<f64>,
    /// True on battery, false on the charger, None without a battery.
    pub on_battery: Option<bool>,
}

/// Where the live systems of Debian mount the medium they started from.
const LIVE_MOUNTS: [&str; 3] = ["/run/live/medium", "/run/initramfs/live", "/lib/live/mount/medium"];

/// The value of KEY="value" in a line of `lsblk -P`.
fn pair<'a>(line: &'a str, key: &str) -> &'a str {
    let tag = format!("{key}=\"");
    line.split_once(&format!(" {tag}"))
        .or_else(|| line.strip_prefix(&tag).map(|rest| ("", rest)))
        .and_then(|(_, rest)| rest.split_once('"'))
        .map(|(value, _)| value)
        .unwrap_or("")
}

/// The size in bytes of the largest disk, leaving out the USB stick the live
/// session runs from, compressed memory (zram), loops and optical drives.
fn largest_disk(lsblk: &str) -> Option<u64> {
    let lines: Vec<&str> = lsblk.lines().collect();
    let live: Vec<&str> = lines
        .iter()
        .filter(|l| LIVE_MOUNTS.contains(&pair(l, "MOUNTPOINT")))
        .map(|l| if pair(l, "TYPE") == "disk" { pair(l, "NAME") } else { pair(l, "PKNAME") })
        .collect();
    lines
        .iter()
        .filter(|l| pair(l, "TYPE") == "disk")
        .filter(|l| {
            let name = pair(l, "NAME");
            !name.starts_with("zram") && !live.contains(&name)
        })
        .filter_map(|l| pair(l, "SIZE").parse::<u64>().ok())
        .max()
}

fn disk_gb() -> Option<f64> {
    let output = Command::new("lsblk")
        .args(["-n", "-b", "-P", "-o", "NAME,PKNAME,SIZE,TYPE,MOUNTPOINT"])
        .output()
        .ok()?;
    largest_disk(&String::from_utf8_lossy(&output.stdout)).map(|bytes| bytes as f64 / 1_000_000_000.0)
}

fn memory_gb() -> Option<f64> {
    let content = fs::read_to_string("/proc/meminfo").ok()?;
    content
        .lines()
        .find(|line| line.starts_with("MemTotal"))
        .and_then(|line| line.split_whitespace().nth(1))
        .and_then(|kb| kb.parse::<f64>().ok())
        .map(|kb| kb / (1024.0 * 1024.0))
}

/// Welcome Live's battery_state: Some(true) when there is a battery and no
/// charger is plugged in.
fn battery_state() -> Option<bool> {
    let entries = fs::read_dir("/sys/class/power_supply").ok()?;
    let mut has_battery = false;
    let mut mains_online = false;
    for entry in entries.flatten() {
        let kind = fs::read_to_string(entry.path().join("type")).unwrap_or_default();
        match kind.trim() {
            "Battery" => has_battery = true,
            "Mains" => {
                let online = fs::read_to_string(entry.path().join("online")).unwrap_or_default();
                if online.trim() == "1" {
                    mains_online = true;
                }
            }
            _ => {}
        }
    }
    if has_battery {
        Some(!mains_online)
    } else {
        None
    }
}

#[tauri::command(async)]
pub fn check_requirements() -> Requirements {
    let data = edition_data::load(&environment::edition());
    Requirements {
        edition_name: data.name,
        disk_gb: disk_gb(),
        disk_required_gb: data.requirements.disk_gb,
        memory_gb: memory_gb(),
        memory_required_gb: data.requirements.memory_gb,
        on_battery: battery_state(),
    }
}

/// Welcome Live's probe: an answer of the servers of the distribution proves
/// that names resolve and HTTPS is not blocked. Without curl or without a
/// probe address, the plain connection of its status indicator decides.
#[tauri::command(async)]
pub fn check_internet() -> bool {
    let data = edition_data::load(&environment::edition());
    if let Some(url) = data.internet_probe.as_deref() {
        let answer = Command::new("curl")
            .args(["-sI", "-m", "6", "--connect-timeout", "5", url])
            .stdin(Stdio::null())
            .stderr(Stdio::null())
            .output();
        if let Ok(output) = answer {
            return output.status.success() && output.stdout.starts_with(b"HTTP/");
        }
    }
    let address: SocketAddr = "8.8.8.8:53".parse().expect("static address");
    TcpStream::connect_timeout(&address, Duration::from_secs(3)).is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_live_stick_is_not_a_target() {
        let lsblk = concat!(
            "NAME=\"sda\" PKNAME=\"\" SIZE=\"500107862016\" TYPE=\"disk\" MOUNTPOINT=\"\"\n",
            "NAME=\"sda1\" PKNAME=\"sda\" SIZE=\"500106813440\" TYPE=\"part\" MOUNTPOINT=\"\"\n",
            "NAME=\"sdb\" PKNAME=\"\" SIZE=\"1000204886016\" TYPE=\"disk\" MOUNTPOINT=\"\"\n",
            "NAME=\"sdb1\" PKNAME=\"sdb\" SIZE=\"4000000000\" TYPE=\"part\" MOUNTPOINT=\"/run/live/medium\"\n",
            "NAME=\"zram0\" PKNAME=\"\" SIZE=\"8000000000000\" TYPE=\"disk\" MOUNTPOINT=\"[SWAP]\"\n",
            "NAME=\"sr0\" PKNAME=\"\" SIZE=\"4000000000\" TYPE=\"rom\" MOUNTPOINT=\"\"\n",
        );
        assert_eq!(largest_disk(lsblk), Some(500_107_862_016));
        assert_eq!(largest_disk(""), None);
    }
}

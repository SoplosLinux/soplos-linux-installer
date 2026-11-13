// The kernel step (DESIGN.md, section 4): the Soplos kernels of the
// repository for the x86-64 level of the processor, read as Soplos Kernel
// Installer 3.0 reads them (commands/soplos_kernels.rs): the names from
// `apt-cache pkgnames linux-soplos`, version and description from
// `apt-cache show`. The level is the helper's own (ld.so, flags as fallback),
// so the window shows exactly what the installation will install.

use crate::helper::kernel::psabi_level;
use serde::Serialize;
use std::collections::HashSet;
use std::process::{Command, Stdio};

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct KernelOption {
    /// The name without "linux-soplos" and the level: "", "lts", "bore-ntsync",
    /// "lts-zen"...
    pub flavor: String,
    pub package: String,
    pub version: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct KernelList {
    /// 1 to 4.
    pub level: u8,
    pub kernels: Vec<KernelOption>,
}

fn apt_cache(args: &[&str]) -> String {
    Command::new("apt-cache")
        .args(args)
        .env("LC_ALL", "C")
        .stdin(Stdio::null())
        .stderr(Stdio::null())
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).into_owned())
        .unwrap_or_default()
}

/// Kernel Installer's valid_kernel_package.
fn valid(name: &str) -> bool {
    let Some(rest) = name.strip_prefix("linux-soplos-") else { return false };
    rest.split('-').all(|part| !part.is_empty() && part.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit()))
}

/// `linux-soplos-lts-zen-v3` with level 3 -> "lts-zen"; another level -> None.
pub fn flavor_of(package: &str, level: u8) -> Option<String> {
    let rest = package.strip_prefix("linux-soplos-")?;
    let tail = format!("v{level}");
    if rest == tail {
        return Some(String::new());
    }
    rest.strip_suffix(&format!("-{tail}")).map(str::to_string)
}

/// The first stanza of each package in `apt-cache show` is its newest
/// version.
fn versions(show: &str) -> Vec<(String, String)> {
    let mut seen = HashSet::new();
    let mut out = Vec::new();
    for block in show.split("\n\n") {
        let field = |key: &str| block.lines().find_map(|l| l.strip_prefix(key)).map(|v| v.trim().to_string()).unwrap_or_default();
        let package = field("Package:");
        if !package.is_empty() && seen.insert(package.clone()) {
            out.push((package, field("Version:")));
        }
    }
    out
}

fn read(level: u8) -> Vec<KernelOption> {
    let names: Vec<String> = apt_cache(&["pkgnames", "linux-soplos"])
        .lines()
        .map(|l| l.trim().to_string())
        .filter(|n| valid(n) && flavor_of(n, level).is_some())
        .collect();
    if names.is_empty() {
        return Vec::new();
    }
    let mut args = vec!["show"];
    args.extend(names.iter().map(String::as_str));
    let mut list: Vec<KernelOption> = versions(&apt_cache(&args))
        .into_iter()
        .filter_map(|(package, version)| Some(KernelOption { flavor: flavor_of(&package, level)?, package, version }))
        .collect();
    list.sort_by(|a, b| a.flavor.cmp(&b.flavor));
    list
}

#[tauri::command(async)]
pub fn get_kernels() -> KernelList {
    let level = psabi_level();
    let mut kernels = read(level);
    // The live image may come without the package lists: fetch them once,
    // with the same rights the helper uses (DESIGN.md, section 2).
    if kernels.is_empty() {
        let updated = Command::new("sudo")
            .args(["-n", "apt-get", "update", "-q"])
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .is_ok_and(|s| s.success());
        if updated {
            kernels = read(level);
        }
    }
    KernelList { level, kernels }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn flavors_of_one_level() {
        assert_eq!(flavor_of("linux-soplos-v3", 3).as_deref(), Some(""));
        assert_eq!(flavor_of("linux-soplos-lts-zen-v3", 3).as_deref(), Some("lts-zen"));
        assert_eq!(flavor_of("linux-soplos-bore-ntsync-v3", 3).as_deref(), Some("bore-ntsync"));
        assert_eq!(flavor_of("linux-soplos-zen-v2", 3), None);
        assert!(!valid("linux-soplos"));
        assert!(valid("linux-soplos-x3d-v4"));
    }

    #[test]
    fn newest_stanza_wins() {
        let show = "Package: linux-soplos-v3\nVersion: 6.19.2-1\n\nPackage: linux-soplos-v3\nVersion: 6.19.1-1\n\nPackage: linux-soplos-zen-v3\nVersion: 6.19.2-1\n";
        assert_eq!(
            versions(show),
            vec![("linux-soplos-v3".into(), "6.19.2-1".into()), ("linux-soplos-zen-v3".into(), "6.19.2-1".into())]
        );
    }
}

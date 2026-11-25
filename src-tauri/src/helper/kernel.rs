// The kernel: the latest stock kernel of Soplos, built for the x86-64 level of
// the processor (read from the dynamic linker, as Kernel Installer shows it) (linux-soplos-v1 to -v4), from the Soplos Kernels repository
// the image already has enabled, as Soplos Kernel Installer 3.0 installs it
// (commands/soplos_kernels.rs, commands/hardware.rs). The kernel of the image
// is removed once the new one is in: the user does not want two
// (2026-11-13).
//
// The drivers the hardware does not need are removed before it, and the
// missing ones installed after, so DKMS builds only what stays, and builds
// it for the new kernel. If the
// repository cannot be reached or the package is missing, the kernel of the
// image stays and the installation goes on.

use super::disk::TARGET;
use super::exec::{Ctx, Stdin};

/// Kernel Installer's psabi_from_flags: the x86-64 level (1 to 4) the flags
/// of the processor allow.
pub fn psabi_from_flags(flags: &str) -> u8 {
    let set: std::collections::HashSet<&str> = flags.split_whitespace().collect();
    if set.is_empty() {
        return 1;
    }
    let has_all = |list: &[&str]| list.iter().all(|f| set.contains(f));
    if !has_all(&["cx16", "lahf_lm", "popcnt", "sse4_1", "sse4_2", "ssse3"]) {
        return 1;
    }
    if !has_all(&["avx", "avx2", "bmi1", "bmi2", "f16c", "fma", "abm", "movbe", "xsave"]) {
        return 2;
    }
    if !has_all(&["avx512f", "avx512bw", "avx512cd", "avx512dq", "avx512vl"]) {
        return 3;
    }
    4
}

/// The level the dynamic linker reports as supported ("x86-64-v3
/// (supported, searched)"), as Kernel Installer shows it. It checks what
/// the processor really allows, also inside a virtual machine.
pub fn psabi_from_ld(help: &str) -> Option<u8> {
    [4u8, 3, 2].into_iter().find(|level| {
        let marker = format!("x86-64-v{level}");
        help.lines().any(|l| l.trim_start().starts_with(&marker) && l.contains("(supported"))
    })
}

pub fn psabi_level() -> u8 {
    for ld in ["/lib64/ld-linux-x86-64.so.2", "/lib/x86_64-linux-gnu/ld-linux-x86-64.so.2"] {
        if let Ok(out) = std::process::Command::new(ld).arg("--help").output() {
            let text = String::from_utf8_lossy(&out.stdout).to_string() + &String::from_utf8_lossy(&out.stderr);
            if text.contains("glibc-hwcaps") {
                return psabi_from_ld(&text).unwrap_or(1);
            }
        }
    }
    // No dynamic linker answer: the flags of the processor.
    let cpuinfo = std::fs::read_to_string("/proc/cpuinfo").unwrap_or_default();
    let flags = cpuinfo
        .lines()
        .find(|l| l.starts_with("flags"))
        .and_then(|l| l.split_once(':'))
        .map(|(_, v)| v.to_string())
        .unwrap_or_default();
    psabi_from_flags(&flags)
}

/// The kernel releases in /lib/modules of the target.
fn releases(ctx: &Ctx) -> Vec<String> {
    if ctx.dry_run {
        return vec!["<kernel of the image>".into()];
    }
    std::fs::read_dir(format!("{TARGET}/lib/modules"))
        .map(|d| {
            d.flatten()
                .filter(|e| e.path().join("modules.dep").exists() || std::path::Path::new(&format!("{TARGET}/boot/vmlinuz-{}", e.file_name().to_string_lossy())).exists())
                .filter_map(|e| e.file_name().into_string().ok())
                .collect()
        })
        .unwrap_or_default()
}

/// The packages of a kernel release: its image (whoever owns the vmlinuz)
/// and its headers when they are installed.
fn packages_of(ctx: &mut Ctx, release: &str) -> Vec<String> {
    let mut out = Vec::new();
    if let Ok(owner) = ctx.read("chroot", &[TARGET, "dpkg", "-S", &format!("/boot/vmlinuz-{release}")]) {
        if let Some((package, _)) = owner.trim().split_once(':') {
            out.push(package.trim().to_string());
        }
    }
    let headers = format!("linux-headers-{release}");
    if ctx
        .read("chroot", &[TARGET, "dpkg-query", "-W", "-f=${Status}", &headers])
        .is_ok_and(|s| s.contains("install ok installed"))
    {
        out.push(headers);
    }
    out
}

/// The Soplos kernel metapackages installed in the target (linux-soplos-*).
fn metapackages(ctx: &mut Ctx) -> Vec<String> {
    if ctx.dry_run {
        return vec!["<metapackage of the kernel of the image>".into()];
    }
    ctx.read("chroot", &[TARGET, "dpkg-query", "-W", "--showformat=${Package} ${Status}\\n", "linux-soplos-*"])
        .unwrap_or_default()
        .lines()
        .filter(|l| l.ends_with("install ok installed"))
        .filter_map(|l| l.split_whitespace().next().map(str::to_string))
        .collect()
}

/// The package of a flavor for a level: ("lts-zen", 3) -> linux-soplos-lts-zen-v3.
/// A flavor that is not a plain name falls back to the stock kernel.
pub fn package_for(flavor: &str, level: u8) -> String {
    let clean = !flavor.is_empty()
        && flavor.split('-').all(|part| !part.is_empty() && part.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit()));
    if clean {
        format!("linux-soplos-{flavor}-v{level}")
    } else {
        format!("linux-soplos-v{level}")
    }
}

/// The whole step: the kernel chosen in the window (the stock one by
/// default), the stock one if that fails, the kernel of the image if both do.
pub fn install(ctx: &mut Ctx, flavor: &str) {
    let level = psabi_level();
    let chosen = package_for(flavor, level);
    let stock = package_for("", level);
    ctx.log(&format!("Processor: x86-64-v{level} -> {chosen}"));
    let before = releases(ctx);

    let mut kept = chosen.clone();
    let mut installed = ctx.run_progress("chroot", &[TARGET, "apt-get", "-y", "install", &chosen], 0, 0);
    if let Err(e) = &installed {
        if chosen != stock {
            ctx.log(&format!("WARNING: {chosen} was not installed ({e}); trying {stock}."));
            installed = ctx.run_progress("chroot", &[TARGET, "apt-get", "-y", "install", &stock], 0, 0);
            kept = stock.clone();
        }
    }
    if let Err(e) = installed {
        ctx.log(&format!("WARNING: the Soplos kernel was not installed ({e}); the kernel of the image stays."));
        return;
    }

    // The kernel of the image goes, once a new one is there.
    let after = releases(ctx);
    let new: Vec<String> = after.iter().filter(|r| !before.contains(r)).cloned().collect();
    if new.is_empty() && !ctx.dry_run {
        ctx.log("The kernel of the image is already the latest Soplos kernel: nothing to replace.");
        return;
    }
    // apt must not stop to ask about removing the kernel the live session
    // runs: it is not the installed system's.
    let _ = ctx.chroot(
        &["debconf-set-selections"],
        Some(Stdin::Text("linux-base linux-base/removing-running-kernel boolean false\n".into())),
    );
    let mut old: Vec<String> = Vec::new();
    for release in &before {
        old.extend(packages_of(ctx, release));
    }
    // The metapackage of the image (linux-soplos-v1...) goes too: left alone,
    // apt upgrades it when the repository has a newer kernel of its flavor and
    // the purge brings that kernel back next to the chosen one.
    old.extend(metapackages(ctx).into_iter().filter(|p| *p != kept));
    if ctx.dry_run {
        old = vec!["<packages of the kernel of the image>".into()];
    }
    if !old.is_empty() {
        let mut args = vec![TARGET, "apt-get", "-y", "purge"];
        args.extend(old.iter().map(String::as_str));
        if let Err(e) = ctx.run_progress("chroot", &args, 0, 0) {
            ctx.log(&format!("WARNING: the kernel of the image could not be removed ({e})."));
            return;
        }
    }
    // What apt does not own (modules DKMS built for it).
    for release in &before {
        let _ = ctx.run("rm", &["-rf", &format!("{TARGET}/lib/modules/{release}")], None);
    }
}

#[cfg(test)]
mod tests {
    use super::{package_for, psabi_from_flags, psabi_from_ld};

    #[test]
    fn packages() {
        assert_eq!(package_for("", 3), "linux-soplos-v3");
        assert_eq!(package_for("lts-zen", 2), "linux-soplos-lts-zen-v2");
        assert_eq!(package_for("zen; rm -rf /", 3), "linux-soplos-v3");
    }

    #[test]
    fn ld_levels() {
        let help = "Subdirectories of glibc-hwcaps directories, in priority order:\n  x86-64-v4\n  x86-64-v3 (supported, searched)\n  x86-64-v2 (supported, searched)\n";
        assert_eq!(psabi_from_ld(help), Some(3));
        assert_eq!(psabi_from_ld("  x86-64-v4\n  x86-64-v3\n  x86-64-v2\n"), None);
    }

    #[test]
    fn psabi_levels() {
        let v2 = "cx16 lahf_lm popcnt sse4_1 sse4_2 ssse3";
        let v3 = format!("{v2} avx avx2 bmi1 bmi2 f16c fma abm movbe xsave");
        let v4 = format!("{v3} avx512f avx512bw avx512cd avx512dq avx512vl");
        assert_eq!(psabi_from_flags(""), 1);
        assert_eq!(psabi_from_flags("sse2"), 1);
        assert_eq!(psabi_from_flags(v2), 2);
        assert_eq!(psabi_from_flags(&v3), 3);
        assert_eq!(psabi_from_flags(&v4), 4);
    }
}

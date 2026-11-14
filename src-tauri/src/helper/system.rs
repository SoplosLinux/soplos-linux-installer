// Steps 3 to 11 of DESIGN.md, section 6: the system is copied from the live
// squashfs into /target and turned into the user's installed system. What
// differs between editions comes from their data file (edition_data.rs),
// the work the Calamares shellprocess scripts did until now.

use super::disk::{Volumes, TARGET};
use super::exec::{Ctx, Stdin};
use super::plan::Plan;
use crate::edition_data::EditionData;
use std::path::Path;

fn t(path: &str) -> String {
    format!("{TARGET}{path}")
}

/// Where Debian's live systems keep the image of the system.
const SQUASHFS: [&str; 3] = [
    "/run/live/medium/live/filesystem.squashfs",
    "/run/initramfs/live/live/filesystem.squashfs",
    "/lib/live/mount/medium/live/filesystem.squashfs",
];

/// Step 3: unpacks the live image into /target, with its real progress.
pub fn copy_system(ctx: &mut Ctx, from: u8, to: u8) -> Result<(), String> {
    let source = SQUASHFS
        .iter()
        .find(|p| Path::new(p).exists())
        .copied()
        .or(if ctx.dry_run { Some(SQUASHFS[0]) } else { None })
        .ok_or("filesystem.squashfs of the live session was not found")?;
    ctx.run_progress("unsquashfs", &["-f", "-percentage", "-d", TARGET, source], from, to)
}

/// The API file systems the commands run in /target need, and the network.
pub fn enter(ctx: &mut Ctx, uefi: bool) -> Result<(), String> {
    ctx.run("mount", &["--rbind", "/dev", &t("/dev")], None)?;
    ctx.run("mount", &["--make-rslave", &t("/dev")], None)?;
    ctx.run("mount", &["-t", "proc", "proc", &t("/proc")], None)?;
    ctx.run("mount", &["--rbind", "/sys", &t("/sys")], None)?;
    ctx.run("mount", &["--make-rslave", &t("/sys")], None)?;
    ctx.run("mount", &["--bind", "/run", &t("/run")], None)?;
    if uefi && !Path::new(&t("/sys/firmware/efi/efivars")).exists() && !ctx.dry_run {
        let _ = ctx.run("mount", &["-t", "efivarfs", "efivarfs", &t("/sys/firmware/efi/efivars")], None);
    }
    // apt needs to resolve names: the live session's resolver, for the
    // duration of the installation. The image's own file comes back at the end.
    let resolv = t("/etc/resolv.conf");
    if Path::new(&resolv).exists() || Path::new(&resolv).is_symlink() {
        ctx.run("mv", &["-f", &resolv, &t("/etc/resolv.conf.sli")], None)?;
    }
    ctx.run("cp", &["-L", "/etc/resolv.conf", &resolv], None)?;
    Ok(())
}

/// The UUID of a file system or LUKS container (blkid). A dry run shows a
/// placeholder, since the devices do not exist yet.
fn uuid(ctx: &mut Ctx, dev: &str) -> Result<String, String> {
    if ctx.dry_run {
        return Ok(format!("<UUID of {dev}>"));
    }
    let out = ctx.read("blkid", &["-s", "UUID", "-o", "value", dev])?;
    let value = out.trim().to_string();
    if value.is_empty() { Err(format!("{dev} has no UUID")) } else { Ok(value) }
}

fn partuuid(ctx: &mut Ctx, dev: &str) -> Result<String, String> {
    if ctx.dry_run {
        return Ok(format!("<PARTUUID of {dev}>"));
    }
    let out = ctx.read("blkid", &["-s", "PARTUUID", "-o", "value", dev])?;
    let value = out.trim().to_string();
    if value.is_empty() { Err(format!("{dev} has no PARTUUID")) } else { Ok(value) }
}

/// What the boot loaders need to know about the root and the swap.
#[derive(Default)]
pub struct BootInfo {
    pub root_uuid: String,
    pub btrfs: bool,
    /// rd.luks.uuid= for each container the initrd has to open.
    pub luks: Vec<String>,
    /// resume= when the swap can hibernate.
    pub resume: Option<String>,
    /// Other parameters: nvidia-drm.modeset=1 when the NVIDIA driver is in.
    pub extra: Vec<String>,
}

/// Step 4a: machine-id, fstab and crypttab (DESIGN.md, sections 5.4 and 7).
/// `hibernate`: the user chose swap for hibernation, so the kernel is told
/// where to resume from.
pub fn write_tables(ctx: &mut Ctx, volumes: &Volumes, hibernate: bool) -> Result<BootInfo, String> {
    let mut info = BootInfo::default();

    ctx.run("rm", &["-f", &t("/etc/machine-id"), &t("/var/lib/dbus/machine-id")], None)?;
    ctx.chroot(&["systemd-machine-id-setup"], None)?;
    ctx.run("mkdir", &["-p", &t("/var/lib/dbus")], None)?;
    ctx.run("ln", &["-sf", "/etc/machine-id", &t("/var/lib/dbus/machine-id")], None)?;

    let mut fstab = String::from("# /etc/fstab, written by Soplos Linux Installer.\n# <file system> <mount point> <type> <options> <dump> <pass>\n");
    let mut crypttab = String::new();

    for (name, part) in &volumes.mapped {
        if name == "soplos-swap" {
            continue;
        }
        let luks = uuid(ctx, part)?;
        crypttab.push_str(&format!("{name} UUID={luks} none luks,discard\n"));
        info.luks.push(luks);
    }

    for (source, mount, fs, options) in &volumes.mounts {
        let id = uuid(ctx, source)?;
        if mount == "/" {
            info.root_uuid = id.clone();
            info.btrfs = fs == "btrfs";
        }
        let pass = if mount == "/" { 1 } else if fs == "vfat" || fs == "ext4" { 2 } else { 0 };
        fstab.push_str(&format!("UUID={id} {mount} {fs} defaults,{options} 0 {pass}\n"));
    }

    if let Some((device, part, kind)) = &volumes.swap {
        match kind.as_str() {
            "plain" => {
                let id = uuid(ctx, device)?;
                fstab.push_str(&format!("UUID={id} none swap sw 0 0\n"));
                if hibernate {
                    info.resume = Some(format!("UUID={id}"));
                }
            }
            "random" => {
                // A new key at every boot: by PARTUUID, the file system is gone.
                let id = partuuid(ctx, part)?;
                crypttab.push_str(&format!("soplos-swap PARTUUID={id} /dev/urandom swap,cipher=aes-xts-plain64,size=512\n"));
                fstab.push_str("/dev/mapper/soplos-swap none swap sw 0 0\n");
            }
            _ => {
                let luks = uuid(ctx, part)?;
                crypttab.push_str(&format!("soplos-swap UUID={luks} none luks,discard\n"));
                fstab.push_str("/dev/mapper/soplos-swap none swap sw 0 0\n");
                info.luks.push(luks);
                info.resume = Some("/dev/mapper/soplos-swap".into());
            }
        }
    }

    ctx.write(&t("/etc/fstab"), &fstab)?;
    if !crypttab.is_empty() {
        ctx.write(&t("/etc/crypttab"), &crypttab)?;
        // The initrd has to open the containers: dracut's crypt module.
        ctx.write(&t("/etc/dracut.conf.d/90-soplos-crypt.conf"), "add_dracutmodules+=\" crypt \"\n")?;
    }
    Ok(info)
}

/// Layouts with another alphabet: an English one goes next to them, or the
/// user could not type a Latin password.
const NON_LATIN: [&str; 16] = ["ru", "ua", "by", "bg", "gr", "il", "ara", "ir", "th", "mk", "rs", "kz", "kg", "mn", "am", "ge"];

/// Step 4b: time zone, language and formats, keyboard and computer name.
pub fn configure(ctx: &mut Ctx, plan: &Plan) -> Result<(), String> {
    // Time zone.
    if !plan.timezone.is_empty() {
        ctx.run("ln", &["-sf", &format!("/usr/share/zoneinfo/{}", plan.timezone), &t("/etc/localtime")], None)?;
        ctx.write(&t("/etc/timezone"), &format!("{}\n", plan.timezone))?;
    }

    // Language and formats: both generated, LANG for the language and the
    // LC_* of dates, numbers and money for the formats.
    let locale = if plan.locale.is_empty() { "en_US.UTF-8".to_string() } else { plan.locale.clone() };
    let formats = if plan.formats.is_empty() { locale.clone() } else { plan.formats.clone() };
    let wanted: Vec<String> = [&locale, &formats].iter().map(|l| format!("{l} UTF-8")).collect();
    ctx.edit(&t("/etc/locale.gen"), &format!("enable {}", wanted.join(", ")), |text| {
        let mut lines: Vec<String> = text
            .lines()
            .map(|line| {
                let bare = line.trim_start_matches('#').trim();
                if wanted.iter().any(|w| w == bare) { bare.to_string() } else { line.to_string() }
            })
            .collect();
        for w in &wanted {
            if !lines.iter().any(|l| l == w) {
                lines.push(w.clone());
            }
        }
        lines.join("\n") + "\n"
    })?;
    ctx.chroot(&["locale-gen"], None)?;
    let short = locale.split('.').next().unwrap_or("en_US").to_string();
    let language = short.split('_').next().unwrap_or("en").to_string();
    let mut default_locale = format!("LANG={locale}\nLANGUAGE={short}:{language}\n");
    for var in ["LC_NUMERIC", "LC_TIME", "LC_MONETARY", "LC_PAPER", "LC_NAME", "LC_ADDRESS", "LC_TELEPHONE", "LC_MEASUREMENT", "LC_IDENTIFICATION"] {
        default_locale.push_str(&format!("{var}={formats}\n"));
    }
    ctx.write(&t("/etc/default/locale"), &default_locale)?;

    // Keyboard: the console and X11 read /etc/default/keyboard; systemd's
    // localed (Plasma, GNOME) reads the xorg.conf.d file.
    let layout = if plan.keyboard.layout.is_empty() { "us".to_string() } else { plan.keyboard.layout.clone() };
    let (layouts, variants, options) = if NON_LATIN.contains(&layout.as_str()) {
        (format!("{layout},us"), format!("{},", plan.keyboard.variant), "grp:alt_shift_toggle".to_string())
    } else {
        (layout.clone(), plan.keyboard.variant.clone(), String::new())
    };
    ctx.write(
        &t("/etc/default/keyboard"),
        &format!(
            "XKBMODEL=\"pc105\"\nXKBLAYOUT=\"{layouts}\"\nXKBVARIANT=\"{variants}\"\nXKBOPTIONS=\"{options}\"\nBACKSPACE=\"guess\"\n"
        ),
    )?;
    ctx.write(
        &t("/etc/X11/xorg.conf.d/00-keyboard.conf"),
        &format!(
            "Section \"InputClass\"\n    Identifier \"system-keyboard\"\n    MatchIsKeyboard \"on\"\n    Option \"XkbLayout\" \"{layouts}\"\n    Option \"XkbModel\" \"pc105\"\n    Option \"XkbVariant\" \"{variants}\"\n    Option \"XkbOptions\" \"{options}\"\nEndSection\n"
        ),
    )?;
    ctx.write(&t("/etc/vconsole.conf"), &format!("KEYMAP={layout}\n"))?;

    // Computer name.
    let host = &plan.user.hostname;
    ctx.write(&t("/etc/hostname"), &format!("{host}\n"))?;
    ctx.write(
        &t("/etc/hosts"),
        &format!("127.0.0.1\tlocalhost\n127.0.1.1\t{host}\n\n::1\tlocalhost ip6-localhost ip6-loopback\nff02::1\tip6-allnodes\nff02::2\tip6-allrouters\n"),
    )?;
    Ok(())
}

/// The groups of the live user in the production image, without disk and
/// autologin (DESIGN.md, section 6.5).
const GROUPS: [&str; 9] = ["sudo", "adm", "cdrom", "audio", "video", "plugdev", "users", "netdev", "lp"];

/// Step 5: the user, the passwords, and the live user gone.
pub fn users(ctx: &mut Ctx, plan: &Plan) -> Result<(), String> {
    let u = &plan.user;
    if u.login.is_empty() || u.password.is_empty() {
        return Err("the plan has no user or no password".into());
    }
    // The live user first, so its uid is free and nothing of it is left.
    let passwd = std::fs::read_to_string(t("/etc/passwd")).unwrap_or_default();
    if ctx.dry_run || passwd.lines().any(|l| l.starts_with("liveuser:")) {
        let _ = ctx.chroot(&["userdel", "-f", "-r", "liveuser"], None);
    }
    ctx.run("rm", &["-rf", &t("/home/liveuser")], None)?;

    let existing = std::fs::read_to_string(t("/etc/group")).unwrap_or_default();
    let groups: Vec<&str> = GROUPS
        .iter()
        .copied()
        .filter(|g| ctx.dry_run || existing.lines().any(|l| l.starts_with(&format!("{g}:"))))
        .collect();
    let groups = groups.join(",");
    ctx.chroot(&["useradd", "-m", "-s", "/bin/bash", "-c", &u.full_name, "-G", &groups, &u.login], None)?;

    let root = if u.same_root_password { &u.password } else { &u.root_password };
    ctx.chroot(&["chpasswd"], Some(Stdin::Secret(format!("{}:{}\nroot:{}\n", u.login, u.password, root))))?;

    // The XDG folders (Documents, Downloads...) in the system's language, as
    // the setup-desktop.sh of Calamares made them: a login shell starts with
    // a clean environment (without the LC_ALL=C of the helper, which made
    // the folders English and asked to rename them at the first login) and
    // reads /etc/default/locale, written in step 4b.
    if let Err(e) = ctx.chroot(&["su", "-", &u.login, "-c", "xdg-user-dirs-update"], None) {
        ctx.log(&format!("WARNING: the XDG folders were not created ({e}); they are created at the first login."));
    }
    ctx.run("chmod", &["755", &t(&format!("/home/{}", u.login))], None)?;
    Ok(())
}

/// Step 6: the display manager of the edition: no live autologin, and what
/// each one needs (DESIGN.md, section 10).
pub fn display_manager(ctx: &mut Ctx, edition: &EditionData, plan: &Plan) -> Result<(), String> {
    match edition.display_manager.as_str() {
        "lightdm" => {
            ctx.edit(&t("/etc/lightdm/lightdm.conf"), "no live autologin", |text| {
                text.lines()
                    .map(|l| if l.trim() == "autologin-user=liveuser" { "#autologin-user=".to_string() } else { l.to_string() })
                    .collect::<Vec<_>>()
                    .join("\n")
                    + "\n"
            })?;
            if let Some(bg) = &edition.lightdm_background {
                let data = t("/var/lib/lightdm/data");
                ctx.run("mkdir", &["-p", &data], None)?;
                ctx.run("cp", &[&t(bg), &format!("{data}/background")], None)?;
                ctx.run("touch", &[&format!("{data}/.first_boot")], None)?;
                ctx.chroot(&["chown", "-R", "lightdm:lightdm", "/var/lib/lightdm/data"], None)?;
                ctx.run("chmod", &["644", &format!("{data}/background")], None)?;
            }
        }
        "gdm3" => {
            ctx.edit(&t("/etc/gdm3/daemon.conf"), "no live autologin", |text| {
                text.lines()
                    .filter(|l| !matches!(l.trim(), "AutomaticLogin=liveuser" | "TimedLogin=liveuser") && !l.trim().starts_with("TimedLoginDelay="))
                    .map(|l| {
                        let trimmed = l.trim();
                        if trimmed.starts_with("AutomaticLoginEnable=") {
                            "AutomaticLoginEnable=false".to_string()
                        } else if trimmed.starts_with("TimedLoginEnable=") {
                            "TimedLoginEnable=false".to_string()
                        } else {
                            l.to_string()
                        }
                    })
                    .collect::<Vec<_>>()
                    .join("\n")
                    + "\n"
            })?;
        }
        "plasmalogin" => {
            ctx.chroot(&["systemctl", "enable", "plasmalogin.service"], None)?;
            // The greeter speaks the system's language (fix-plasmalogin-locale.sh).
            let short = plan.locale.split('.').next().unwrap_or("").to_string();
            let language = short.split('_').next().unwrap_or("").to_string();
            if !short.is_empty() {
                let list = if short == language { language } else { format!("{short}:{language}") };
                ctx.write(&t("/var/lib/plasmalogin/.config/plasma-localerc"), &format!("[Translations]\nLANGUAGE={list}\n"))?;
                let _ = ctx.chroot(&["chown", "-R", "plasmalogin:plasmalogin", "/var/lib/plasmalogin/.config"], None);
            }
        }
        _ => {}
    }
    Ok(())
}

/// Seconds since the epoch of the `Date:` header of an HTTP answer.
fn server_epoch(headers: &str) -> Option<i64> {
    let line = headers.lines().find(|l| l.to_ascii_lowercase().starts_with("date:"))?;
    let value = line.split_once(':')?.1.trim();
    let parts: Vec<&str> = value.split_whitespace().collect();
    let day: i64 = parts.get(1)?.parse().ok()?;
    let month = ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"]
        .iter()
        .position(|m| m.eq_ignore_ascii_case(parts.get(2).copied().unwrap_or("")))? as i64
        + 1;
    let year: i64 = parts.get(3)?.parse().ok()?;
    let mut time = parts.get(4)?.split(':');
    let (h, m, s): (i64, i64, i64) = (time.next()?.parse().ok()?, time.next()?.parse().ok()?, time.next()?.parse().ok()?);
    let y = if month <= 2 { year - 1 } else { year };
    let era = y.div_euclid(400);
    let yoe = y.rem_euclid(400);
    let doy = (153 * ((month + 9) % 12) + 2) / 5 + day - 1;
    let days = era * 146097 + yoe * 365 + yoe / 4 - yoe / 100 + doy - 719468;
    Some(days * 86400 + h * 3600 + m * 60 + s)
}

/// apt refuses repositories whose dates are "in the future": a live session
/// with a wrong clock sets it from the servers first, as Welcome Live does.
fn fix_clock(ctx: &mut Ctx, probe: &str) {
    let Ok(headers) = ctx.read("curl", &["-sI", "-m", "6", "--connect-timeout", "5", probe]) else { return };
    let Some(server) = server_epoch(&headers) else { return };
    let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs() as i64).unwrap_or(0);
    if (now - server).abs() > 300 {
        let _ = ctx.run("date", &["-u", "-s", &format!("@{server}")], None);
        let _ = ctx.run("hwclock", &["--systohc"], None);
    }
}

fn installed(ctx: &mut Ctx, packages: &[String]) -> Vec<String> {
    if ctx.dry_run {
        return packages.to_vec();
    }
    packages
        .iter()
        .filter(|p| {
            ctx.read("chroot", &[TARGET, "dpkg-query", "-W", "-f=${Status}", p.as_str()])
                .is_ok_and(|s| s.contains("install ok installed"))
        })
        .cloned()
        .collect()
}

/// Step 7: the live packages out, soplos-welcome in (from the internet).
pub fn packages(ctx: &mut Ctx, edition: &EditionData, from: u8, to: u8) -> Result<(), String> {
    if let Some(probe) = edition.internet_probe.clone() {
        fix_clock(ctx, &probe);
    }
    ctx.run_progress("chroot", &[TARGET, "apt-get", "update"], from, from)?;
    // initramfs-tools removed but not purged ("rc") leaves its kernel hook
    // in /etc/kernel/postinst.d, which runs next to dracut's and leaves
    // duplicate initrds. Soplos uses dracut: the leftovers are purged, as was
    // done by hand on the machines that build the images.
    let status = if ctx.dry_run {
        "rc ".to_string()
    } else {
        ctx.read("chroot", &[TARGET, "dpkg-query", "-W", "-f=${db:Status-Abbrev}", "initramfs-tools"]).unwrap_or_default()
    };
    if status.starts_with("rc") {
        ctx.chroot(&["dpkg", "--purge", "initramfs-tools"], None)?;
    }
    let remove = installed(ctx, &edition.remove_packages);
    if !remove.is_empty() {
        let mut args = vec![TARGET, "apt-get", "-y", "purge"];
        args.extend(remove.iter().map(String::as_str));
        ctx.run_progress("chroot", &args, from, from)?;
    }
    if !edition.install_packages.is_empty() {
        let mut args = vec![TARGET, "apt-get", "-y", "install"];
        args.extend(edition.install_packages.iter().map(String::as_str));
        ctx.run_progress("chroot", &args, to, to)?;
    }
    Ok(())
}

/// The updates of the repositories, when the user asked for them (kernel
/// step of the window). It runs after the new kernel is in and the old one
/// gone, so no update is fetched for the kernel that leaves, and before the
/// drivers, so DKMS builds against the system already up to date. The
/// configuration files the image ships are kept. A failed update does not
/// stop the installation: the system is left consistent and Soplos updates
/// it later.
pub fn update(ctx: &mut Ctx, from: u8, to: u8) {
    let args = [
        TARGET,
        "apt-get",
        "-y",
        "-o",
        "Dpkg::Options::=--force-confdef",
        "-o",
        "Dpkg::Options::=--force-confold",
        "full-upgrade",
    ];
    if let Err(e) = ctx.run_progress("chroot", &args, from, to) {
        ctx.log(&format!("WARNING: the system was not updated ({e})."));
        let _ = ctx.chroot(&["dpkg", "--configure", "-a"], None);
    }
}

/// The kernel command line every boot loader starts from.
fn cmdline(info: &BootInfo, with_root: bool) -> String {
    let mut parts: Vec<String> = Vec::new();
    if with_root {
        parts.push(format!("root=UUID={} ro", info.root_uuid));
        if info.btrfs {
            parts.push("rootflags=subvol=@".into());
        }
    }
    for luks in &info.luks {
        parts.push(format!("rd.luks.uuid={luks}"));
    }
    if let Some(resume) = &info.resume {
        parts.push(format!("resume={resume}"));
    }
    parts.extend(info.extra.iter().cloned());
    parts.push("quiet splash".into());
    parts.join(" ")
}

/// GRUB packages, purged when another boot loader is chosen, so a later
/// GRUB upgrade cannot take the first place of the boot order.
const GRUB_PACKAGES: [&str; 9] = [
    "grub-efi-amd64",
    "grub-efi-amd64-signed",
    "grub-efi-amd64-bin",
    "grub-pc",
    "grub-pc-bin",
    "grub-common",
    "grub2-common",
    "shim-signed",
    "shim-signed-common",
];

fn purge_grub(ctx: &mut Ctx) -> Result<(), String> {
    let all: Vec<String> = GRUB_PACKAGES.iter().map(|s| s.to_string()).collect();
    let present = installed(ctx, &all);
    if present.is_empty() {
        return Ok(());
    }
    let mut args = vec![TARGET, "apt-get", "-y", "--allow-remove-essential", "purge"];
    args.extend(present.iter().map(String::as_str));
    ctx.run_progress("chroot", &args, 0, 0).map(|_| ()).or_else(|_| Ok(()))
}

/// Steps 9 and 10: the kernel command line, the initramfs and the boot loader.
pub fn bootloader(ctx: &mut Ctx, plan: &Plan, edition: &EditionData, info: &BootInfo, alongside: bool) -> Result<(), String> {
    let uefi = plan.firmware != "bios";
    ctx.write(&t("/etc/kernel/cmdline"), &format!("{}\n", cmdline(info, true)))?;
    ctx.write(&t("/etc/kernel/entry-token"), &format!("{}\n", edition.codename))?;

    match plan.bootloader.as_str() {
        "systemd-boot" => {
            ctx.run_progress("chroot", &[TARGET, "apt-get", "-y", "install", "systemd-boot"], 0, 0)?;
            ctx.chroot(&["bootctl", "install", "--esp-path=/boot/efi"], None)?;
            purge_grub(ctx)?;
            ctx.chroot(&["update-initramfs", "-u", "-k", "all"], None)?;
            // Every kernel of the image gets its entry.
            for version in kernels(ctx) {
                ctx.chroot(&["kernel-install", "add", &version, &format!("/boot/vmlinuz-{version}")], None)?;
            }
        }
        "limine" => {
            ctx.run_progress(
                "chroot",
                &[TARGET, "apt-get", "-y", "install", "limine", "limine-soplos", "soplos-limine-theme"],
                0,
                0,
            )?;
            ctx.chroot(&["update-initramfs", "-u", "-k", "all"], None)?;
            ctx.chroot(&["limine-install", "--force"], None)?;
            purge_grub(ctx)?;
        }
        _ => {
            // The image's own /etc/default/grub stays as it is: it holds the
            // Soplos theme and its graphical terminal. Calamares does the same
            // (grub.cfg.conf: overwrite false, always_use_defaults false, so its
            // defaults only apply when the file is missing). Only what the
            // installation needs is added: the LUKS and resume parameters,
            // and os-prober when another system is on the disk.
            let extra = cmdline(info, false).replace("quiet splash", "").trim().to_string();
            let os_prober = alongside;
            ctx.edit(&t("/etc/default/grub"), "LUKS/resume parameters and os-prober", move |text| {
                let mut lines: Vec<String> = text.lines().map(String::from).collect();
                if !extra.is_empty() {
                    let key = "GRUB_CMDLINE_LINUX=";
                    match lines.iter().position(|l| l.trim_start().starts_with(key)) {
                        Some(i) => {
                            let current = lines[i].trim_start()[key.len()..].trim_matches('"').trim().to_string();
                            let joined = [current, extra.clone()].iter().filter(|s| !s.is_empty()).cloned().collect::<Vec<_>>().join(" ");
                            lines[i] = format!("{key}\"{joined}\"");
                        }
                        None => lines.push(format!("{key}\"{extra}\"")),
                    }
                }
                if os_prober {
                    match lines.iter().position(|l| l.trim_start_matches('#').trim_start().starts_with("GRUB_DISABLE_OS_PROBER=")) {
                        Some(i) => lines[i] = "GRUB_DISABLE_OS_PROBER=false".into(),
                        None => lines.push("GRUB_DISABLE_OS_PROBER=false".into()),
                    }
                }
                lines.join("\n") + "\n"
            })?;
            ctx.chroot(&["update-initramfs", "-u", "-k", "all"], None)?;
            if uefi {
                ctx.chroot(
                    &[
                        "grub-install",
                        "--target=x86_64-efi",
                        "--efi-directory=/boot/efi",
                        &format!("--bootloader-id={}", edition.codename),
                        "--force-extra-removable",
                        "--recheck",
                    ],
                    None,
                )?;
            } else {
                ctx.chroot(&["grub-install", "--target=i386-pc", "--recheck", &plan.disk.device], None)?;
            }
            ctx.chroot(&["update-grub"], None)?;
        }
    }
    Ok(())
}

/// The kernel versions installed in /target (from /lib/modules).
fn kernels(ctx: &mut Ctx) -> Vec<String> {
    if ctx.dry_run {
        return vec!["<kernel version>".into()];
    }
    std::fs::read_dir(t("/lib/modules"))
        .map(|dir| dir.flatten().filter_map(|e| e.file_name().into_string().ok()).collect())
        .unwrap_or_default()
}

/// Step 11: the live leftovers out, the resolver and the log back.
/// The mark Soplos Welcome Live leaves in the live session when the user
/// turns NumLock off in its tools (Tyron).
const NUMLOCK_OFF_MARK: &str = "/etc/soplos-linux-installer/numlock-off";

/// NumLock off, as the Tyron Calamares did it (disable-numlockx and
/// packages.conf): the greeter no longer turns it on, and numlockx goes.
fn numlock_off(ctx: &mut Ctx) -> Result<(), String> {
    ctx.log("NumLock: turned off in Soplos Welcome Live.");
    let lightdm = t("/etc/lightdm/lightdm.conf");
    if ctx.dry_run || Path::new(&lightdm).exists() {
        ctx.edit(&lightdm, "NumLock off in the greeter", |text| {
            let lines: Vec<String> = text
                .lines()
                .map(|l| if l.starts_with("greeter-setup-script=/usr/bin/numlockx on") { format!("#{l}") } else { l.to_string() })
                .collect();
            lines.join("\n") + "\n"
        })?;
    }
    if !installed(ctx, &["numlockx".to_string()]).is_empty() {
        ctx.run_progress("chroot", &[TARGET, "apt-get", "-y", "purge", "numlockx"], 0, 0)?;
    }
    Ok(())
}

pub fn finish(ctx: &mut Ctx, edition: &EditionData) -> Result<(), String> {
    if Path::new(NUMLOCK_OFF_MARK).exists() {
        numlock_off(ctx)?;
    }
    for file in &edition.remove_files {
        ctx.run("rm", &["-rf", &t(file)], None)?;
    }
    let saved = t("/etc/resolv.conf.sli");
    ctx.run("rm", &["-f", &t("/etc/resolv.conf")], None)?;
    if ctx.dry_run || Path::new(&saved).exists() || Path::new(&saved).is_symlink() {
        ctx.run("mv", &["-f", &saved, &t("/etc/resolv.conf")], None)?;
    }
    if !ctx.dry_run {
        let _ = std::fs::copy(super::exec::LOG_FILE, t("/var/log/soplos-linux-installer.log"));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn command_lines() {
        let info = BootInfo {
            root_uuid: "1234".into(),
            btrfs: true,
            luks: vec!["aaaa".into()],
            resume: Some("/dev/mapper/soplos-swap".into()),
            extra: vec![],
        };
        assert_eq!(
            cmdline(&info, true),
            "root=UUID=1234 ro rootflags=subvol=@ rd.luks.uuid=aaaa resume=/dev/mapper/soplos-swap quiet splash"
        );
        assert_eq!(cmdline(&BootInfo::default(), false), "quiet splash");
    }

    #[test]
    fn the_date_of_a_server() {
        assert_eq!(server_epoch("HTTP/2 200\r\nDate: Sun, 08 Nov 2026 10:00:00 GMT\r\n"), Some(1_794_132_000));
    }
}

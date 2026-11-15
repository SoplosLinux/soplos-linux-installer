// Steps 1 and 2 of DESIGN.md, section 6: the disk ends with exactly the table
// the window showed (plan.disk.layout), the file systems are written, the
// encrypted ones opened, and everything is mounted at /target.
//
// The table is written with sfdisk (util-linux), which handles GPT and MBR,
// new tables and changes to existing ones.

use super::exec::{Ctx, Stdin};
use super::plan::{Layout, Part, Plan, GUID_BASIC_DATA, GUID_BIOS_GRUB, GUID_ESP, GUID_SWAP, GUID_XBOOTLDR};
use serde::Deserialize;
use std::fs;

pub const TARGET: &str = "/target";

/// An existing partition as the disk has it now.
#[derive(Debug, Clone)]
pub struct Existing {
    pub path: String,
    pub number: u32,
    /// Bytes from the start of the disk.
    pub start: u64,
    pub parttype: String,
    pub mounted: bool,
}

#[derive(Debug, Clone)]
pub struct DiskState {
    pub sector: u64,
    /// "gpt", "dos" or "".
    pub table: String,
    pub parts: Vec<Existing>,
    pub rotational: bool,
}

#[derive(Deserialize)]
struct LsblkRoot {
    blockdevices: Vec<LsblkDev>,
}

#[derive(Deserialize)]
struct LsblkDev {
    path: String,
    name: String,
    #[serde(rename = "type")]
    kind: String,
    start: Option<u64>,
    pttype: Option<String>,
    parttype: Option<String>,
    #[serde(default)]
    mountpoints: Vec<Option<String>>,
    #[serde(rename = "log-sec")]
    log_sec: Option<u64>,
    #[serde(default)]
    children: Vec<LsblkDev>,
}

/// The disk as it is now. Only reads, also in a dry run.
pub fn read_state(ctx: &mut Ctx, device: &str) -> Result<DiskState, String> {
    let json = ctx.read("lsblk", &["-J", "-b", "-o", "PATH,NAME,TYPE,START,PTTYPE,PARTTYPE,MOUNTPOINTS,LOG-SEC", device])?;
    let root: LsblkRoot = serde_json::from_str(&json).map_err(|e| format!("lsblk: {e}"))?;
    let disk = root.blockdevices.into_iter().find(|d| d.kind == "disk").ok_or("the device is not a disk")?;
    let sector = disk.log_sec.unwrap_or(512);
    let parts = disk
        .children
        .iter()
        .filter(|c| c.kind == "part")
        .map(|c| Existing {
            path: c.path.clone(),
            number: fs::read_to_string(format!("/sys/class/block/{}/partition", c.name))
                .ok()
                .and_then(|n| n.trim().parse().ok())
                .unwrap_or(0),
            start: c.start.unwrap_or(0) * sector,
            parttype: c.parttype.clone().unwrap_or_default().to_lowercase(),
            mounted: c.mountpoints.iter().flatten().next().is_some(),
        })
        .collect();
    let rotational = fs::read_to_string(format!("/sys/block/{}/queue/rotational", disk.name))
        .map(|v| v.trim() != "0")
        .unwrap_or(true);
    Ok(DiskState { sector, table: disk.pttype.unwrap_or_default(), parts, rotational })
}

/// /dev/sdb + 2 -> /dev/sdb2; /dev/nvme0n1 + 2 -> /dev/nvme0n1p2.
pub fn partition_path(device: &str, number: u32) -> String {
    if device.ends_with(|c: char| c.is_ascii_digit()) {
        format!("{device}p{number}")
    } else {
        format!("{device}{number}")
    }
}

/// The partition type as sfdisk takes it: the GUID on GPT, a code on MBR.
fn sfdisk_type(guid: &str, gpt: bool) -> String {
    if gpt {
        return guid.to_uppercase();
    }
    match guid.to_lowercase().as_str() {
        GUID_ESP => "ef",
        GUID_SWAP => "82",
        GUID_BASIC_DATA => "c",
        GUID_XBOOTLDR => "ea",
        _ => "83",
    }
    .to_string()
}

/// A partition of the plan with the device it ends up as.
#[derive(Debug, Clone)]
pub struct Placed {
    pub part: Part,
    pub dev: String,
}

/// Writes the table of the plan. Returns every partition with its device.
pub fn partition(ctx: &mut Ctx, plan: &Plan, state: &DiskState) -> Result<Vec<Placed>, String> {
    let device = plan.disk.device.as_str();
    let layout: &Layout = plan.disk.layout.as_ref().ok_or("the plan has no partition table")?;
    let ss = state.sector;
    let sectors = |bytes: u64| bytes / ss;

    if !device.starts_with("/dev/") {
        return Err(format!("not a device: {device}"));
    }
    if state.parts.iter().any(|p| p.mounted) {
        return Err("a partition of the disk is in use (mounted or swap): unmount it first".into());
    }
    // The plan was made from the disk as it was then: the partitions it keeps
    // must still be where they were.
    for part in layout.parts.iter().filter(|p| !p.path.is_empty()) {
        if !state.parts.iter().any(|e| e.path == part.path && e.start == part.start) {
            return Err(format!("{} is not where it was: the disk changed, go back to the disk step", part.path));
        }
    }

    let mut numbers: Vec<(usize, u32)> = Vec::new(); // index in layout -> number
    if layout.new_table {
        let gpt = layout.table.as_deref() != Some("msdos");
        if !gpt && layout.parts.len() > 4 {
            return Err("an MBR table holds 4 partitions at most".into());
        }
        let mut script = format!("label: {}\n", if gpt { "gpt" } else { "dos" });
        for part in &layout.parts {
            script.push_str(&format!(
                "start={}, size={}, type={}\n",
                sectors(part.start),
                sectors(part.size),
                sfdisk_type(&part.parttype, gpt)
            ));
        }
        ctx.run("wipefs", &["-a", "-f", device], None)?;
        ctx.run("sfdisk", &["--wipe", "always", "--wipe-partitions", "always", device], Some(Stdin::Text(script)))?;
        numbers = (0..layout.parts.len()).map(|i| (i, i as u32 + 1)).collect();
    } else {
        let gpt = state.table == "gpt";
        if !gpt && layout.parts.len() > 4 {
            return Err("an MBR table holds 4 partitions at most; extended partitions are not supported yet".into());
        }
        // Deleted: what the disk has and the plan does not.
        let deleted: Vec<&Existing> =
            state.parts.iter().filter(|e| !layout.parts.iter().any(|p| p.path == e.path)).collect();
        if !deleted.is_empty() {
            let mut args = vec!["--delete".to_string(), device.to_string()];
            args.extend(deleted.iter().map(|e| e.number.to_string()));
            let refs: Vec<&str> = args.iter().map(String::as_str).collect();
            ctx.run("sfdisk", &refs, None)?;
        }
        let mut used: Vec<u32> =
            state.parts.iter().filter(|e| !deleted.iter().any(|d| d.path == e.path)).map(|e| e.number).collect();
        for (i, part) in layout.parts.iter().enumerate() {
            let Some(existing) = state.parts.iter().find(|e| e.path == part.path) else { continue };
            numbers.push((i, existing.number));
            let n = existing.number.to_string();
            if part.action == "shrink" {
                shrink_filesystem(ctx, part)?;
                ctx.run(
                    "sfdisk",
                    &["--no-reread", "-N", &n, device],
                    Some(Stdin::Text(format!("{},{}\n", sectors(part.start), sectors(part.size)))),
                )?;
            }
            // A partition taken for another use gets its type (an ESP mounted
            // at /boot/efi, a swap...).
            let wanted = sfdisk_type(&part.parttype, gpt);
            let current = sfdisk_type(&existing.parttype, gpt);
            if (part.action == "use" || part.action == "format") && !part.parttype.is_empty() && wanted != current {
                ctx.run("sfdisk", &["--part-type", device, &n, &wanted], None)?;
            }
        }
        let new: Vec<(usize, &Part)> = layout.parts.iter().enumerate().filter(|(_, p)| p.path.is_empty()).collect();
        if !new.is_empty() {
            let mut script = String::new();
            for (i, part) in &new {
                script.push_str(&format!(
                    "start={}, size={}, type={}\n",
                    sectors(part.start),
                    sectors(part.size),
                    sfdisk_type(&part.parttype, gpt)
                ));
                // sfdisk gives each new partition the lowest free number.
                let number = (1..).find(|n| !used.contains(n)).unwrap_or(1);
                used.push(number);
                numbers.push((*i, number));
            }
            ctx.run("sfdisk", &["--append", device], Some(Stdin::Text(script)))?;
        }
    }

    ctx.run("udevadm", &["settle"], None)?;

    // The device of each partition: predicted from its number, and in a real
    // run checked against what the kernel now shows, by where it starts.
    let after = if ctx.dry_run { None } else { Some(read_state(ctx, device)?) };
    let mut placed = Vec::new();
    for (i, part) in layout.parts.iter().enumerate() {
        let dev = match &after {
            Some(state) => state
                .parts
                .iter()
                .find(|e| e.start == part.start)
                .map(|e| e.path.clone())
                .ok_or_else(|| format!("the partition at {} MiB did not appear", part.start / (1024 * 1024)))?,
            None => {
                let number = numbers.iter().find(|(j, _)| *j == i).map(|(_, n)| *n).unwrap_or(0);
                if part.path.is_empty() { partition_path(device, number) } else { part.path.clone() }
            }
        };
        placed.push(Placed { part: part.clone(), dev });
    }
    Ok(placed)
}

/// Makes the file system of a partition that is going to shrink fit in its
/// new size, before the partition itself is made smaller.
fn shrink_filesystem(ctx: &mut Ctx, part: &Part) -> Result<(), String> {
    let size = part.size.to_string();
    match part.fs.as_str() {
        "ntfs" => {
            ctx.run("ntfsresize", &["--force", "--no-action", "--size", &size, &part.path], None)?;
            ctx.run("ntfsresize", &["--force", "--size", &size, &part.path], Some(Stdin::Text("y\n".into())))?;
        }
        "ext2" | "ext3" | "ext4" => {
            ctx.run_allowing("e2fsck", &["-f", "-y", &part.path], 1)?;
            ctx.run("resize2fs", &[&part.path, &format!("{}K", part.size / 1024)], None)?;
        }
        "btrfs" => {
            let dir = "/run/soplos-linux-installer/shrink";
            ctx.run("mkdir", &["-p", dir], None)?;
            ctx.run("mount", &["-t", "btrfs", &part.path, dir], None)?;
            let result = ctx.run("btrfs", &["filesystem", "resize", &size, dir], None);
            ctx.run("umount", &[dir], None)?;
            result?;
        }
        other => return Err(format!("{other} cannot be shrunk")),
    }
    Ok(())
}

/// What the later steps need to know: where each file system is.
#[derive(Debug, Clone, Default)]
pub struct Volumes {
    /// (source device, mount point, file system, options), in mount order.
    pub mounts: Vec<(String, String, String, String)>,
    /// Opened LUKS devices: (mapper name, partition).
    pub mapped: Vec<(String, String)>,
    /// The swap: (device or mapper, partition, kind) with kind "plain",
    /// "random" (a new key at every boot) or "luks" (the same passphrase).
    pub swap: Option<(String, String, String)>,
    pub ssd: bool,
}

fn mapper_name(mount: &str) -> &'static str {
    match mount {
        "/" => "soplos-root",
        "/home" => "soplos-home",
        _ => "soplos-data",
    }
}

fn label_for(part: &Part) -> &'static str {
    match (part.fs.as_str(), part.mount.as_str()) {
        ("vfat", "/boot/efi") => "EFI",
        ("vfat", "/boot/limine") => "LIMINE",
        ("vfat", "/boot") => "BOOT",
        (_, "/") => "Soplos",
        (_, "/home") => "Home",
        (_, "/boot") => "Boot",
        _ => "",
    }
}

fn luks_open(ctx: &mut Ctx, plan: &Plan, dev: &str, name: &str, volumes: &mut Volumes) -> Result<String, String> {
    let secret = || Some(Stdin::Secret(plan.disk.passphrase.clone()));
    ctx.run("cryptsetup", &["luksFormat", "--type", "luks2", "--batch-mode", "--key-file=-", dev], secret())?;
    ctx.run("cryptsetup", &["open", "--key-file=-", dev, name], secret())?;
    volumes.mapped.push((name.to_string(), dev.to_string()));
    Ok(format!("/dev/mapper/{name}"))
}

/// The file system already on a partition, "" when it has none or it cannot be read.
fn fs_type(ctx: &mut Ctx, dev: &str) -> String {
    ctx.read("blkid", &["-o", "value", "-s", "TYPE", dev]).map(|s| s.trim().to_string()).unwrap_or_default()
}

/// Writes the file systems, with LUKS under the ones that ask for it
/// (DESIGN.md, 5.4), and mounts everything at /target.
pub fn format_and_mount(ctx: &mut Ctx, plan: &Plan, placed: &[Placed], state: &DiskState) -> Result<Volumes, String> {
    let mut volumes = Volumes { ssd: !state.rotational, ..Default::default() };
    if plan.disk.encrypt && plan.disk.passphrase.is_empty() {
        return Err("encryption was chosen without a passphrase".into());
    }

    // Formatting.
    let mut sources: Vec<(usize, String)> = Vec::new();
    for (i, p) in placed.iter().enumerate() {
        let part = &p.part;
        if part.parttype.to_lowercase() == GUID_BIOS_GRUB {
            continue;
        }
        if part.fs == "swap" {
            if !part.writes() && part.action != "use" {
                continue;
            }
            // The swap of the system next to Soplos, shared as it is, also
            // when Soplos is encrypted: mkswap would give it a new UUID, and
            // a key would format it at every start, and in both cases that
            // system would lose its swap.
            if part.action == "use" && fs_type(ctx, &p.dev) == "swap" {
                volumes.swap = Some((p.dev.clone(), p.dev.clone(), "plain".into()));
                continue;
            }
            if plan.disk.encrypt && plan.disk.swap == "hibernate" {
                let mapper = luks_open(ctx, plan, &p.dev, "soplos-swap", &mut volumes)?;
                ctx.run("mkswap", &["-L", "swap", &mapper], None)?;
                volumes.swap = Some((mapper, p.dev.clone(), "luks".into()));
            } else if plan.disk.encrypt {
                // Formatted at every boot with a random key, from crypttab.
                ctx.run("wipefs", &["-a", &p.dev], None)?;
                volumes.swap = Some((p.dev.clone(), p.dev.clone(), "random".into()));
            } else {
                ctx.run("mkswap", &["-L", "swap", &p.dev], None)?;
                volumes.swap = Some((p.dev.clone(), p.dev.clone(), "plain".into()));
            }
            continue;
        }
        let mut target = p.dev.clone();
        if part.writes() {
            if part.encrypt {
                target = luks_open(ctx, plan, &p.dev, mapper_name(&part.mount), &mut volumes)?;
            }
            let label = label_for(part);
            match part.fs.as_str() {
                "vfat" => ctx.run("mkfs.fat", &["-F", "32", "-n", if label.is_empty() { "DATA" } else { label }, &target], None)?,
                "ext4" => ctx.run("mkfs.ext4", &["-F", "-L", label, &target], None)?,
                "btrfs" => ctx.run("mkfs.btrfs", &["-f", "-L", label, &target], None)?,
                "xfs" => ctx.run("mkfs.xfs", &["-f", "-L", label, &target], None)?,
                other => return Err(format!("unknown file system: {other}")),
            };
        }
        if !part.mount.is_empty() {
            sources.push((i, target));
        }
    }

    // Mounting: / first, then by depth (/boot before /boot/efi).
    sources.sort_by_key(|(i, _)| {
        let mount = &placed[*i].part.mount;
        if mount == "/" { 0 } else { mount.matches('/').count() + 1 }
    });
    if !sources.iter().any(|(i, _)| placed[*i].part.mount == "/") {
        return Err("the plan has no root partition".into());
    }
    let separate_home = sources.iter().any(|(i, _)| placed[*i].part.mount == "/home");
    ctx.run("mkdir", &["-p", TARGET], None)?;
    for (i, source) in &sources {
        let part = &placed[*i].part;
        let fs = part.fs.as_str();
        let options = match fs {
            "vfat" => "umask=0077".to_string(),
            "btrfs" => format!("noatime,compress=zstd:1{}", if volumes.ssd { ",discard=async" } else { "" }),
            _ => "noatime".to_string(),
        };
        let dir = if part.mount == "/" { TARGET.to_string() } else { format!("{TARGET}{}", part.mount) };
        if part.mount == "/" && fs == "btrfs" {
            // Subvolumes @ and @home, as Calamares makes them and Timeshift
            // needs (only @ when /home has its own partition).
            ctx.run("mount", &["-t", "btrfs", source, TARGET], None)?;
            ctx.run("btrfs", &["subvolume", "create", &format!("{TARGET}/@")], None)?;
            if !separate_home {
                ctx.run("btrfs", &["subvolume", "create", &format!("{TARGET}/@home")], None)?;
            }
            ctx.run("umount", &[TARGET], None)?;
            let root_options = format!("{options},subvol=@");
            ctx.run("mount", &["-t", "btrfs", "-o", &root_options, source, TARGET], None)?;
            volumes.mounts.push((source.clone(), "/".into(), "btrfs".into(), root_options));
            if !separate_home {
                let home = format!("{TARGET}/home");
                let home_options = format!("{options},subvol=@home");
                ctx.run("mkdir", &["-p", &home], None)?;
                ctx.run("mount", &["-t", "btrfs", "-o", &home_options, source, &home], None)?;
                volumes.mounts.push((source.clone(), "/home".into(), "btrfs".into(), home_options));
            }
            continue;
        }
        ctx.run("mkdir", &["-p", &dir], None)?;
        ctx.run("mount", &["-t", fs, "-o", &options, source, &dir], None)?;
        volumes.mounts.push((source.clone(), part.mount.clone(), fs.to_string(), options));
    }
    Ok(volumes)
}

/// Unmounts /target and closes LUKS. Run also after a failure; errors here
/// are logged and do not hide the first one.
pub fn release(ctx: &mut Ctx, volumes: &Volumes) {
    if ctx.run("umount", &["-R", TARGET], None).is_err() {
        let _ = ctx.run("umount", &["-R", "-l", TARGET], None);
    }
    for (name, _) in volumes.mapped.iter().rev() {
        let _ = ctx.run("cryptsetup", &["close", name], None);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::helper::plan::{DiskPlan, GUID_ESP};
    use std::cell::RefCell;
    use std::rc::Rc;

    const MIB: u64 = 1024 * 1024;
    const GIB: u64 = 1024 * MIB;

    fn part(path: &str, start: u64, size: u64, action: &str, fs: &str, mount: &str, parttype: &str, encrypt: bool) -> Part {
        Part {
            path: path.into(),
            start,
            size,
            action: action.into(),
            fs: fs.into(),
            mount: mount.into(),
            parttype: parttype.into(),
            encrypt,
            label: String::new(),
        }
    }

    /// Runs the steps as a dry run and returns the commands they list.
    fn commands(plan: &Plan, state: &DiskState) -> (Vec<String>, Vec<Placed>) {
        let lines = Rc::new(RefCell::new(Vec::new()));
        let sink = lines.clone();
        let mut ctx = Ctx::new(
            true,
            Box::new(move |e| {
                if let super::super::Event::Log { line } = e {
                    sink.borrow_mut().push(line.clone());
                }
            }),
        );
        let placed = partition(&mut ctx, plan, state).expect("partition");
        format_and_mount(&mut ctx, plan, &placed, state).expect("format");
        let out = lines.borrow().clone();
        (out, placed)
    }

    #[test]
    fn erase_with_encryption() {
        let linux = "0fc63daf-8483-4772-8e79-3d69d8477de4";
        let plan = Plan {
            disk: DiskPlan {
                device: "/dev/sdb".into(),
                encrypt: true,
                passphrase: "secret".into(),
                swap: "swap".into(),
                layout: Some(Layout {
                    new_table: true,
                    table: Some("gpt".into()),
                    parts: vec![
                        part("", MIB, GIB, "create", "vfat", "/boot/efi", GUID_ESP, false),
                        part("", MIB + GIB, 2 * GIB, "create", "ext4", "/boot", linux, false),
                        part("", MIB + 3 * GIB, 30 * GIB, "create", "btrfs", "/", linux, true),
                        part("", MIB + 33 * GIB, 8 * GIB, "create", "swap", "", GUID_SWAP, false),
                    ],
                }),
                ..Default::default()
            },
            ..Default::default()
        };
        let state = DiskState { sector: 512, table: String::new(), parts: vec![], rotational: false };
        let (out, placed) = commands(&plan, &state);
        let all = out.join("\n");
        println!("{all}");
        assert!(all.contains("label: gpt"));
        assert!(all.contains("start=2048, size=2097152, type=C12A7328-F81F-11D2-BA4B-00A0C93EC93B"));
        assert!(!all.contains("secret"), "the passphrase must never be logged");
        assert!(all.contains("cryptsetup luksFormat --type luks2 --batch-mode --key-file=- /dev/sdb3"));
        assert!(all.contains("mkfs.btrfs -f -L Soplos /dev/mapper/soplos-root"));
        assert!(all.contains("subvol=@home"));
        assert!(all.contains("discard=async"));
        assert!(all.contains("wipefs -a /dev/sdb4"), "random-key swap is not formatted now");
        // /boot is mounted before /boot/efi
        let boot = all.find("/dev/sdb2 /target/boot").unwrap();
        let efi = all.find("/target/boot/efi").unwrap();
        assert!(boot < efi);
        assert_eq!(placed[3].dev, "/dev/sdb4");
    }

    #[test]
    fn alongside_windows() {
        let linux = "0fc63daf-8483-4772-8e79-3d69d8477de4";
        let state = DiskState {
            sector: 512,
            table: "gpt".into(),
            rotational: true,
            parts: vec![
                Existing { path: "/dev/nvme0n1p1".into(), number: 1, start: MIB, parttype: GUID_ESP.into(), mounted: false },
                Existing { path: "/dev/nvme0n1p2".into(), number: 2, start: 101 * MIB, parttype: "e3c9e316-0b5c-4db8-817d-f92df00215ae".into(), mounted: false },
                Existing { path: "/dev/nvme0n1p3".into(), number: 3, start: 117 * MIB, parttype: GUID_BASIC_DATA.into(), mounted: false },
            ],
        };
        let plan = Plan {
            disk: DiskPlan {
                device: "/dev/nvme0n1".into(),
                swap: "swap".into(),
                layout: Some(Layout {
                    new_table: false,
                    table: None,
                    parts: vec![
                        part("/dev/nvme0n1p1", MIB, 100 * MIB, "use", "vfat", "/boot/efi", GUID_ESP, false),
                        part("/dev/nvme0n1p2", 101 * MIB, 16 * MIB, "keep", "", "", "", false),
                        part("/dev/nvme0n1p3", 117 * MIB, 200 * GIB, "shrink", "ntfs", "", GUID_BASIC_DATA, false),
                        part("", 117 * MIB + 200 * GIB, 2 * GIB, "create", "ext4", "/boot", linux, false),
                        part("", 117 * MIB + 202 * GIB, 100 * GIB, "create", "ext4", "/", linux, false),
                    ],
                }),
                ..Default::default()
            },
            ..Default::default()
        };
        let (out, placed) = commands(&plan, &state);
        let all = out.join("\n");
        println!("{all}");
        assert!(!all.contains("--delete"));
        assert!(all.contains("ntfsresize --force --no-action --size 214748364800 /dev/nvme0n1p3"));
        assert!(all.contains("sfdisk --no-reread -N 3 /dev/nvme0n1"));
        assert!(all.contains("sfdisk --append /dev/nvme0n1"));
        assert!(!all.contains("mkfs.fat"), "the ESP of Windows is reused, not formatted");
        assert!(!all.contains("discard=async"));
        assert_eq!(placed[3].dev, "/dev/nvme0n1p4");
        assert_eq!(placed[4].dev, "/dev/nvme0n1p5");
    }

    #[test]
    fn a_changed_disk_stops_everything() {
        let state = DiskState { sector: 512, table: "gpt".into(), rotational: true, parts: vec![] };
        let plan = Plan {
            disk: DiskPlan {
                device: "/dev/sdb".into(),
                layout: Some(Layout {
                    new_table: false,
                    table: None,
                    parts: vec![part("/dev/sdb1", MIB, GIB, "keep", "ext4", "", "", false)],
                }),
                ..Default::default()
            },
            ..Default::default()
        };
        let mut ctx = Ctx::new(true, Box::new(|_| {}));
        assert!(partition(&mut ctx, &plan, &state).is_err());
    }
}

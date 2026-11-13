// The disks the system can be installed on, with their partitions, and how
// small an existing partition can get when Soplos is installed alongside it
// (DESIGN.md, section 5). Nothing here writes to a disk: the helper does
// that once the user confirms.

use serde::{Deserialize, Serialize};
use std::io::Write;
use std::process::{Command, Stdio};

#[derive(Deserialize)]
struct LsblkRoot {
    blockdevices: Vec<LsblkDevice>,
}

#[derive(Deserialize)]
struct LsblkDevice {
    path: String,
    name: String,
    #[serde(rename = "type")]
    kind: String,
    size: Option<u64>,
    start: Option<u64>,
    fstype: Option<String>,
    fsver: Option<String>,
    label: Option<String>,
    partlabel: Option<String>,
    parttype: Option<String>,
    #[serde(default)]
    mountpoints: Vec<Option<String>>,
    fsused: Option<u64>,
    model: Option<String>,
    tran: Option<String>,
    #[serde(default)]
    rm: bool,
    #[serde(default)]
    ro: bool,
    pttype: Option<String>,
    #[serde(rename = "log-sec")]
    log_sec: Option<u64>,
    #[serde(default)]
    children: Vec<LsblkDevice>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Partition {
    pub path: String,
    /// Bytes from the start of the disk.
    pub start: u64,
    pub size: u64,
    pub fstype: String,
    pub fsver: String,
    pub label: String,
    pub partlabel: String,
    /// The GPT type GUID or the MBR type ("0x7").
    pub parttype: String,
    pub mountpoints: Vec<String>,
    /// Bytes in use, known only when the live session has it mounted.
    pub fsused: Option<u64>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Disk {
    pub path: String,
    pub model: String,
    pub size: u64,
    /// "sata", "nvme", "usb"...
    pub transport: String,
    pub removable: bool,
    /// "gpt", "dos" or "" with no partition table.
    pub table: String,
    pub sector_size: u64,
    pub partitions: Vec<Partition>,
}

/// Where the live systems of Debian mount the medium they started from.
const LIVE_MOUNTS: [&str; 3] = ["/run/live/medium", "/run/initramfs/live", "/lib/live/mount/medium"];

fn mounts(device: &LsblkDevice) -> Vec<String> {
    device.mountpoints.iter().flatten().cloned().collect()
}

fn parse(json: &str) -> Vec<Disk> {
    let Ok(root) = serde_json::from_str::<LsblkRoot>(json) else { return Vec::new() };
    root.blockdevices
        .into_iter()
        .filter(|d| d.kind == "disk" && !d.ro && !d.name.starts_with("zram"))
        // The stick the live session runs from is never a target.
        .filter(|d| {
            let on_disk = mounts(d).iter().any(|m| LIVE_MOUNTS.contains(&m.as_str()));
            let on_part = d.children.iter().any(|c| mounts(c).iter().any(|m| LIVE_MOUNTS.contains(&m.as_str())));
            !on_disk && !on_part
        })
        .map(|d| {
            let sector = d.log_sec.unwrap_or(512);
            let mut partitions: Vec<Partition> = d
                .children
                .iter()
                .filter(|c| c.kind == "part")
                .map(|c| Partition {
                    path: c.path.clone(),
                    start: c.start.unwrap_or(0) * sector,
                    size: c.size.unwrap_or(0),
                    fstype: c.fstype.clone().unwrap_or_default(),
                    fsver: c.fsver.clone().unwrap_or_default(),
                    label: c.label.clone().unwrap_or_default(),
                    partlabel: c.partlabel.clone().unwrap_or_default(),
                    parttype: c.parttype.clone().unwrap_or_default(),
                    mountpoints: mounts(c),
                    fsused: c.fsused,
                })
                .collect();
            partitions.sort_by_key(|p| p.start);
            Disk {
                path: d.path,
                model: d.model.unwrap_or_default().trim().to_string(),
                size: d.size.unwrap_or(0),
                transport: d.tran.unwrap_or_default(),
                removable: d.rm,
                table: d.pttype.unwrap_or_default(),
                sector_size: sector,
                partitions,
            }
        })
        .collect()
}

#[tauri::command(async)]
pub fn list_disks() -> Vec<Disk> {
    let output = Command::new("lsblk")
        .args([
            "-J",
            "-b",
            "-o",
            "PATH,NAME,TYPE,SIZE,START,FSTYPE,FSVER,LABEL,PARTLABEL,PARTTYPE,MOUNTPOINTS,FSUSED,MODEL,TRAN,RM,RO,PTTYPE,LOG-SEC",
        ])
        .output();
    match output {
        Ok(o) => parse(&String::from_utf8_lossy(&o.stdout)),
        Err(_) => Vec::new(),
    }
}

/// Reads, as root and without changing anything, how much a file system can
/// shrink. It prints "MIN <bytes>" or "ERR <reason>". btrfs has to be mounted
/// to be measured: read-only and without replaying its log.
const PROBE: &str = r#"
export PATH=/usr/sbin:/usr/bin:/sbin:/bin
dev="$1"; fs="$2"
case "$fs" in
  ntfs)
    out=$(ntfsresize --info --force --no-action "$dev" 2>&1)
    if printf '%s' "$out" | grep -qi hibernat; then echo "ERR hibernated"; exit 0; fi
    min=$(printf '%s' "$out" | sed -n 's/.*resize at \([0-9]*\) bytes.*/\1/p' | head -n1)
    if [ -n "$min" ]; then echo "MIN $min"; else echo "ERR unreadable"; fi ;;
  ext2|ext3|ext4)
    bs=$(dumpe2fs -h "$dev" 2>/dev/null | sed -n 's/^Block size: *\([0-9]*\).*/\1/p')
    blocks=$(resize2fs -P "$dev" 2>/dev/null | sed -n 's/.*minimum size of the filesystem: *\([0-9]*\).*/\1/p')
    if [ -n "$bs" ] && [ -n "$blocks" ]; then echo "MIN $((bs * blocks))"; else echo "ERR unreadable"; fi ;;
  btrfs)
    dir=$(mktemp -d)
    if mount -t btrfs -o ro,rescue=nologreplay "$dev" "$dir" 2>/dev/null; then
      used=$(btrfs filesystem usage -b "$dir" 2>/dev/null | awk '/^ *Used:/{print $2; exit}')
      umount "$dir"
      if [ -n "$used" ]; then echo "MIN $used"; else echo "ERR unreadable"; fi
    else
      echo "ERR unreadable"
    fi
    rmdir "$dir" ;;
  *) echo "ERR unsupported" ;;
esac
"#;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ShrinkInfo {
    /// The smallest size the file system fits in, in bytes, before any margin.
    pub min_bytes: Option<u64>,
    /// "hibernated" (Windows hibernation or Fast Startup), "unreadable",
    /// "unsupported" or "nosudo".
    pub error: Option<String>,
}

#[tauri::command(async)]
pub fn partition_min_size(path: String, fstype: String) -> ShrinkInfo {
    let valid_path = path.starts_with("/dev/") && path[5..].chars().all(|c| c.is_ascii_alphanumeric() || "/_-".contains(c));
    let valid_fs = ["ntfs", "ext2", "ext3", "ext4", "btrfs"].contains(&fstype.as_str());
    if !valid_path || !valid_fs {
        return ShrinkInfo { min_bytes: None, error: Some("unsupported".into()) };
    }
    let child = Command::new("sudo")
        .args(["-n", "/bin/bash", "-s", "--", &path, &fstype])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn();
    let Ok(mut child) = child else { return ShrinkInfo { min_bytes: None, error: Some("nosudo".into()) } };
    if let Some(mut stdin) = child.stdin.take() {
        let _ = stdin.write_all(PROBE.as_bytes());
    }
    let Ok(output) = child.wait_with_output() else { return ShrinkInfo { min_bytes: None, error: Some("nosudo".into()) } };
    let text = String::from_utf8_lossy(&output.stdout);
    let line = text.lines().last().unwrap_or("");
    if let Some(bytes) = line.strip_prefix("MIN ").and_then(|b| b.trim().parse().ok()) {
        ShrinkInfo { min_bytes: Some(bytes), error: None }
    } else if let Some(reason) = line.strip_prefix("ERR ") {
        ShrinkInfo { min_bytes: None, error: Some(reason.trim().to_string()) }
    } else {
        ShrinkInfo { min_bytes: None, error: Some(if output.status.success() { "unreadable" } else { "nosudo" }.into()) }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn disks_without_the_live_stick() {
        let json = r#"{"blockdevices":[
          {"path":"/dev/sda","name":"sda","type":"disk","size":500107862016,"start":null,"model":"Samsung SSD ","tran":"sata","rm":false,"ro":false,"pttype":"gpt","log-sec":512,"mountpoints":[null],
           "children":[{"path":"/dev/sda1","name":"sda1","type":"part","size":104857600,"start":2048,"fstype":"vfat","parttype":"c12a7328-f81f-11d2-ba4b-00a0c93ec93b","mountpoints":[null]},
                       {"path":"/dev/sda3","name":"sda3","type":"part","size":400000000000,"start":239616,"fstype":"ntfs","mountpoints":[null]}]},
          {"path":"/dev/sdb","name":"sdb","type":"disk","size":16000000000,"rm":true,"ro":false,"mountpoints":[null],
           "children":[{"path":"/dev/sdb1","name":"sdb1","type":"part","size":4000000000,"start":0,"fstype":"iso9660","mountpoints":["/run/live/medium"]}]},
          {"path":"/dev/zram0","name":"zram0","type":"disk","size":8000000000,"mountpoints":["[SWAP]"]}
        ]}"#;
        let disks = parse(json);
        assert_eq!(disks.len(), 1);
        assert_eq!(disks[0].model, "Samsung SSD");
        assert_eq!(disks[0].partitions[1].start, 239616 * 512);
        assert_eq!(disks[0].partitions[0].mountpoints.len(), 0);
    }
}

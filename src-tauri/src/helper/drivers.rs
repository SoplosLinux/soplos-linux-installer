// Drivers (DESIGN.md, section 8): the hardware scan and the driver flows of
// Soplos Welcome 3.0 (src-tauri/src/commands/hardware_scan.rs, drivers.rs,
// mac.rs), ported with as few changes as possible and run with no questions
// during the installation:
//   - the detection reads the live session, which runs on the same hardware;
//   - what is installed is read from the system being installed;
//   - the scripts run inside it (chroot), with the kernel of the target in
//     $KVER instead of $(uname -r);
//   - they do not touch GRUB or the initramfs: the boot loader step comes
//     after and writes nvidia-drm.modeset=1 where the chosen boot loader
//     reads it, then regenerates the initramfs.
// A driver that fails does not stop the installation: the system boots with
// the free drivers and Soplos Welcome offers the rest.
//
// Changes from Welcome, agreed in DESIGN.md:
//   - NVIDIA: Turing and newer get the 620 branch (pinning + nvidia-open),
//     Maxwell and Pascal the 580 branch. Everything that would need 550 or
//     older stays on nouveau.
//   - Hybrid laptops get the prime-run helper of Welcome's on-demand mode.

use super::disk::TARGET;
use super::exec::{Ctx, Stdin};
use super::nvidia_patches::{nvidia_dkms_patch_commands, LLVM_DKMS_TOOLCHAIN};
use std::collections::HashSet;
use std::path::Path;

// --- Detection (hardware_scan.rs and drivers.rs of Welcome) -----------------

#[derive(Debug, Clone)]
pub struct Gpu {
    pub vendor: String,
    pub model: String,
    /// "nvidia-580", "nvidia-620", "nouveau", or the packages of AMD/Intel.
    pub recommended: String,
}

/// Mirrors Python's `\b`: "ati" must not match inside "Corporation".
fn contains_word_boundary(haystack: &str, needle: &str) -> bool {
    let is_word_char = |c: char| c.is_alphanumeric();
    let mut start = 0;
    while let Some(pos) = haystack[start..].find(needle) {
        let abs_pos = start + pos;
        let before_ok = haystack[..abs_pos].chars().last().map_or(true, |c| !is_word_char(c));
        let after_idx = abs_pos + needle.len();
        let after_ok = haystack[after_idx..].chars().next().map_or(true, |c| !is_word_char(c));
        if before_ok && after_ok {
            return true;
        }
        start = abs_pos + 1;
        if start >= haystack.len() {
            break;
        }
    }
    false
}

const NVIDIA_TURING_PLUS_MARKERS: &[&str] = &[
    "rtx", "gtx 16", "gtx16", "mx450", "mx 450", "mx550", "mx 550", "mx570", "mx 570", "t400", "t500", "t550",
    "t600", "t1000", "t1200", "t2000", "tesla t4",
];

fn extract_bracketed_model(line: &str) -> Option<String> {
    let start = line.rfind('[')?;
    let end = line[start..].find(']')? + start;
    Some(line[start + 1..end].trim().to_string())
}

fn extract_model_fallback(line: &str, patterns: &[&str]) -> Option<String> {
    let lower = line.to_lowercase();
    for pattern in patterns {
        if let Some(pos) = lower.find(pattern) {
            let rest = &line[pos..];
            let end = rest.find('(').unwrap_or(rest.len());
            let candidate = rest[..end].trim();
            let candidate = ["intel ", "amd ", "nvidia "]
                .iter()
                .find(|v| candidate.to_lowercase().starts_with(*v))
                .map(|v| candidate[v.len()..].trim())
                .unwrap_or(candidate);
            if !candidate.is_empty() {
                return Some(candidate.to_string());
            }
        }
    }
    None
}

/// Welcome's recommend_nvidia_driver, with the SLI policy on top: only 580
/// (Maxwell, Pascal) and 620 (Turing and newer) are installed; the rest of
/// the cards stay on nouveau.
fn recommend_nvidia(model: &str) -> &'static str {
    let model_lower = model.to_lowercase();
    // "gtx 16" is the start of a model number (GTX 1650, 1660 Ti): as a whole
    // word it never matches. Welcome has that bug and sends these Turing
    // cards to nouveau; fixed here.
    let gtx16 = ["gtx 16", "gtx16"].iter().any(|m| model_lower.contains(m));
    if gtx16 || NVIDIA_TURING_PLUS_MARKERS.iter().any(|m| contains_word_boundary(&model_lower, m)) {
        return "nvidia-620";
    }
    let pascal_maxwell = [
        "gtx 10", "gtx10", "gtx 1080", "gtx 1070", "gtx 1060", "gtx 1050", "gt 1030", "mx350", "mx 350", "mx330",
        "mx 330", "mx250", "mx 250", "mx150", "mx 150", "gtx 9", "gtx 980", "gtx 970", "gtx 960", "gtx 950", "gtx 8",
        "gtx 880", "gtx 870", "gtx 860", "gtx 850", "920m", "930m", "940m", "920mx", "930mx", "940mx", "910m", "820m",
        "840m", "960m", "970m", "980m",
    ];
    if pascal_maxwell.iter().any(|s| model_lower.contains(s)) {
        return "nvidia-580";
    }
    // Kepler (470), Fermi (390), older, professional (Debian's nvidia-driver,
    // the 550 branch) and unknown cards: nouveau.
    "nouveau"
}

const AMD_PACKAGES: [&str; 5] = ["firmware-amd-graphics", "libgl1-mesa-dri", "libglx-mesa0", "mesa-vulkan-drivers", "xserver-xorg-video-all"];
const INTEL_PACKAGES: [&str; 2] = ["intel-media-va-driver", "mesa-vulkan-drivers"];

pub fn detect_gpus(lspci: &str) -> Vec<Gpu> {
    let mut gpus = Vec::new();
    for line in lspci.lines() {
        let lower = line.to_lowercase();
        if !["vga", "3d", "display"].iter().any(|k| lower.contains(k)) {
            continue;
        }
        if ["vmware svga", "virtualbox graphics", "vbox graphics", "qxl paravirtual", "bochs vbe", "virtio", "red hat"]
            .iter()
            .any(|k| lower.contains(k))
        {
            gpus.push(Gpu { vendor: "Virtual".into(), model: "Virtual GPU".into(), recommended: String::new() });
        } else if contains_word_boundary(&lower, "nvidia") {
            let model = extract_bracketed_model(line)
                .or_else(|| extract_model_fallback(line, &["geforce rtx", "geforce gtx", "geforce", "nvidia"]))
                .unwrap_or_else(|| "NVIDIA Unknown".to_string());
            let recommended = recommend_nvidia(&model).to_string();
            gpus.push(Gpu { vendor: "NVIDIA".into(), model, recommended });
        } else if contains_word_boundary(&lower, "amd") || contains_word_boundary(&lower, "ati") || contains_word_boundary(&lower, "radeon") {
            let model = extract_bracketed_model(line)
                .or_else(|| extract_model_fallback(line, &["radeon rx", "radeon r", "radeon", "amd"]))
                .unwrap_or_else(|| "AMD Unknown".to_string());
            gpus.push(Gpu { vendor: "AMD".into(), model, recommended: AMD_PACKAGES.join(" ") });
        } else if contains_word_boundary(&lower, "intel") {
            let model = extract_bracketed_model(line)
                .or_else(|| extract_model_fallback(line, &["uhd graphics", "hd graphics", "iris xe", "iris", "intel"]))
                .unwrap_or_else(|| "Intel Unknown".to_string());
            gpus.push(Gpu { vendor: "Intel".into(), model, recommended: INTEL_PACKAGES.join(" ") });
        }
    }
    gpus
}

/// Welcome's detect_hybrid_gpu: NVIDIA plus an Intel or an integrated AMD GPU.
fn is_hybrid(gpus: &[Gpu]) -> bool {
    let is_discrete_amd = |model: &str| {
        let lower = model.to_lowercase();
        ["rx ", "vega", "vii", "radeon pro"].iter().any(|m| lower.contains(m))
    };
    gpus.iter().any(|g| g.vendor == "NVIDIA")
        && gpus.iter().any(|g| g.vendor == "Intel" || (g.vendor == "AMD" && !is_discrete_amd(&g.model)))
}

fn identify_wifi_vendor(line_lower: &str) -> (Option<&'static str>, Option<&'static str>) {
    if line_lower.contains("intel") {
        (Some("Intel"), Some("firmware-iwlwifi"))
    } else if line_lower.contains("realtek") {
        (Some("Realtek"), Some("firmware-realtek"))
    } else if line_lower.contains("broadcom") || line_lower.contains("bcm") {
        (Some("Broadcom"), Some("firmware-b43-installer"))
    } else if line_lower.contains("atheros") || line_lower.contains("qualcomm") {
        (Some("Atheros/Qualcomm"), Some("firmware-atheros"))
    } else if line_lower.contains("ralink") || line_lower.contains("mediatek") {
        (Some("Ralink/MediaTek"), Some("firmware-mediatek"))
    } else if line_lower.contains("marvell") {
        (Some("Marvell"), Some("firmware-libertas"))
    } else {
        (None, None)
    }
}

/// The Wi-Fi vendors present and their firmware packages.
fn detect_wifi(lspci: &str, lsusb: &str) -> Vec<(String, Option<String>)> {
    let keywords = ["wireless", "wifi", "802.11", "wlan", "wi-fi"];
    let mut seen = HashSet::new();
    let mut out = Vec::new();
    for line in lspci.lines().chain(lsusb.lines()) {
        let lower = line.to_lowercase();
        if !keywords.iter().any(|k| lower.contains(k)) {
            continue;
        }
        let (vendor, firmware) = identify_wifi_vendor(&lower);
        let vendor = vendor.unwrap_or("Unknown");
        if seen.insert(vendor.to_string()) {
            out.push((vendor.to_string(), firmware.map(String::from)));
        }
    }
    out
}

#[derive(Debug, Clone, Default)]
pub struct Vm {
    pub is_vm: bool,
    /// "VirtualBox", "VMware", "QEMU/KVM", "Hyper-V" or the raw name.
    pub kind: String,
    pub required: Vec<String>,
}

/// Welcome's detect_vm, on the live session (the same machine).
fn detect_vm(ctx: &mut Ctx) -> Vm {
    let mut vm_raw: Option<String> = None;
    if let Ok(text) = ctx.read("systemd-detect-virt", &[]) {
        let text = text.trim().to_lowercase();
        if text != "none" && !text.is_empty() {
            vm_raw = Some(text);
        }
    }
    if vm_raw.is_none() {
        for dmi_file in ["/sys/class/dmi/id/sys_vendor", "/sys/class/dmi/id/product_name"] {
            if let Ok(content) = std::fs::read_to_string(dmi_file) {
                let content = content.to_lowercase();
                if content.contains("vmware") {
                    vm_raw = Some("vmware".into());
                } else if content.contains("virtualbox") {
                    vm_raw = Some("oracle".into());
                } else if content.contains("qemu") || content.contains("kvm") {
                    vm_raw = Some("kvm".into());
                }
                if vm_raw.is_some() {
                    break;
                }
            }
        }
    }
    let Some(raw) = vm_raw else { return Vm::default() };
    let (kind, required): (&str, Vec<&str>) = if raw.contains("oracle") || raw.contains("virtualbox") {
        ("VirtualBox", vec![])
    } else if raw.contains("vmware") {
        ("VMware", vec!["open-vm-tools", "open-vm-tools-desktop"])
    } else if raw.contains("kvm") || raw.contains("qemu") {
        ("QEMU/KVM", vec!["qemu-guest-agent", "spice-vdagent"])
    } else if raw.contains("microsoft") {
        ("Hyper-V", vec!["hyperv-daemons"])
    } else {
        (raw.as_str(), vec![])
    };
    Vm { is_vm: true, kind: kind.to_string(), required: required.iter().map(|s| s.to_string()).collect() }
}

fn read_dmi(name: &str) -> String {
    std::fs::read_to_string(format!("/sys/class/dmi/id/{name}")).map(|s| s.trim().to_string()).unwrap_or_default()
}

fn facetime_camera_present() -> bool {
    let Ok(devices) = std::fs::read_dir("/sys/bus/pci/devices") else { return false };
    devices.flatten().any(|device| {
        let read = |file: &str| std::fs::read_to_string(device.path().join(file)).unwrap_or_default().trim().to_lowercase();
        read("vendor") == "0x14e4" && read("device") == "0x1570"
    })
}

/// What the installation does with the drivers: removals first, then the
/// packages, then the flows that need scripts.
#[derive(Debug, Default)]
pub struct DriverPlan {
    /// Plain apt purges (Welcome's "generic" findings).
    pub purge: Vec<String>,
    /// NVIDIA/CUDA installed with no NVIDIA GPU: Welcome's uninstall_nvidia.
    pub purge_nvidia: bool,
    /// VirtualBox Guest Additions enabled outside VirtualBox.
    pub remove_vbox: bool,
    /// Packages missing for the hardware present.
    pub install: Vec<String>,
    /// "580" or "620".
    pub nvidia: Option<String>,
    pub hybrid: bool,
    pub vbox_additions: bool,
    pub mac_tlp: bool,
    pub facetimehd: bool,
    /// One line per device, for the log.
    pub summary: Vec<String>,
}

fn starts_with_any(pkg: &str, prefixes: &[&str]) -> bool {
    let lower = pkg.to_lowercase();
    prefixes.iter().any(|p| lower.starts_with(p))
}

/// Builds the plan from what the hardware has and what the target has.
pub fn plan(lspci: &str, lsusb: &str, installed: &[String], vm: Vm, vbox_enabled: bool, mac: (bool, String, bool)) -> DriverPlan {
    let has = |p: &str| installed.iter().any(|i| i.eq_ignore_ascii_case(p));
    let mut plan = DriverPlan::default();
    let mut want: Vec<String> = Vec::new();
    let need = |list: &[&str], want: &mut Vec<String>| {
        for p in list {
            if !has(p) && !want.iter().any(|w| w == p) {
                want.push(p.to_string());
            }
        }
    };

    let gpus = detect_gpus(lspci);
    let wifi = detect_wifi(lspci, lsusb);
    for gpu in &gpus {
        plan.summary.push(format!("GPU: {} {} -> {}", gpu.vendor, gpu.model, if gpu.recommended.is_empty() { "-" } else { &gpu.recommended }));
        match gpu.vendor.as_str() {
            "NVIDIA" => {
                if let Some(branch) = gpu.recommended.strip_prefix("nvidia-") {
                    plan.nvidia = Some(branch.to_string());
                }
            }
            "AMD" => need(&AMD_PACKAGES, &mut want),
            "Intel" => need(&INTEL_PACKAGES, &mut want),
            _ => {}
        }
    }
    plan.hybrid = plan.nvidia.is_some() && is_hybrid(&gpus);

    for (vendor, firmware) in &wifi {
        plan.summary.push(format!("Wi-Fi: {vendor}"));
        if let Some(firmware) = firmware {
            need(&[firmware.as_str()], &mut want);
        }
    }

    // Audio: ALSA and a sound server (PipeWire counts).
    let audio = lspci
        .lines()
        .any(|l| ["audio", "sound", "multimedia audio", "high definition audio", "ac97"].iter().any(|k| l.to_lowercase().contains(k)));
    if audio {
        need(&["alsa-utils"], &mut want);
        if !(has("pulseaudio") || has("pipewire-pulse") || has("pipewire")) {
            need(&["pulseaudio"], &mut want);
        }
    }
    if lsusb.to_lowercase().contains("bluetooth") || lspci.to_lowercase().contains("bluetooth") {
        need(&["bluetooth", "bluez"], &mut want);
    }
    if lsusb.lines().any(|l| l.to_lowercase().contains("printer")) {
        need(&["cups", "printer-driver-all"], &mut want);
    }

    if vm.is_vm {
        plan.summary.push(format!("Virtual machine: {}", vm.kind));
        let required: Vec<&str> = vm.required.iter().map(String::as_str).collect();
        need(&required, &mut want);
        plan.vbox_additions = vm.kind == "VirtualBox" && !vbox_enabled;
    }

    let (is_mac, model, facetime) = mac;
    if is_mac {
        plan.summary.push(format!("Mac: {model}"));
        if model.starts_with("MacBook") {
            need(&["mbpfan"], &mut want);
            plan.mac_tlp = !has("tlp");
        }
        plan.facetimehd = facetime && !has("facetimehd-dkms");
    }

    // Welcome's detect_unnecessary_software.
    let gpu_vendors: HashSet<&str> = gpus.iter().map(|g| g.vendor.as_str()).collect();
    let wifi_vendors: HashSet<&str> = wifi.iter().map(|(v, _)| v.as_str()).collect();
    if !gpu_vendors.contains("NVIDIA") && installed.iter().any(|p| starts_with_any(p, &["nvidia", "libnvidia", "cuda"])) {
        plan.purge_nvidia = true;
    }
    if !gpu_vendors.contains("AMD") {
        for p in ["xserver-xorg-video-amdgpu", "xserver-xorg-video-ati", "xserver-xorg-video-radeon", "radeontop"] {
            if has(p) {
                plan.purge.push(p.into());
            }
        }
        plan.purge.extend(installed.iter().filter(|p| starts_with_any(p, &["rocm"])).cloned());
    }
    if !wifi_vendors.contains("Broadcom") {
        for p in ["broadcom-sta-dkms", "bcmwl-kernel-source", "firmware-b43-installer"] {
            if has(p) {
                plan.purge.push(p.into());
            }
        }
    }
    if !vm.is_vm {
        for p in ["open-vm-tools", "open-vm-tools-desktop", "qemu-guest-agent", "spice-vdagent", "spice-webdavd", "hyperv-daemons"] {
            if has(p) {
                plan.purge.push(p.into());
            }
        }
    }
    // On a VM, the tools of the other hypervisors are not needed either.
    if vm.is_vm {
        let others: &[&str] = match vm.kind.as_str() {
            "VMware" => &["qemu-guest-agent", "spice-vdagent", "spice-webdavd", "hyperv-daemons"],
            "QEMU/KVM" => &["open-vm-tools", "open-vm-tools-desktop", "hyperv-daemons"],
            "Hyper-V" => &["open-vm-tools", "open-vm-tools-desktop", "qemu-guest-agent", "spice-vdagent", "spice-webdavd"],
            "VirtualBox" => &["open-vm-tools", "open-vm-tools-desktop", "qemu-guest-agent", "spice-vdagent", "spice-webdavd", "hyperv-daemons"],
            _ => &[],
        };
        for p in others {
            if has(p) {
                plan.purge.push(p.to_string());
            }
        }
    }
    plan.remove_vbox = vbox_enabled && vm.kind != "VirtualBox";

    plan.install = want.into_iter().filter(|p| !plan.purge.contains(p)).collect();
    plan
}

// --- Scripts (drivers.rs and mac.rs of Welcome, run in the chroot) -----------

/// Welcome's uninstall_nvidia, without GRUB and initramfs (done later).
const NVIDIA_UNINSTALL: &str = r#"echo "=== Removing all NVIDIA drivers ==="
for entry in $(dkms status 2>/dev/null | grep -i nvidia | awk -F'[:,]' '{print $1"/"$2}' | tr -d ' '); do
    dkms remove --force "$entry" --all 2>/dev/null || true
done
for kver in $(ls /lib/modules/); do
    rm -f /lib/modules/$kver/updates/dkms/nvidia*.ko*
    depmod -a "$kver" 2>/dev/null || true
done
rm -rf /var/lib/dkms/nvidia*
dpkg --configure -a 2>/dev/null || true
NVIDIA_PKGS=$(dpkg -l | grep -iE '^[a-z]+[[:space:]]+(nvidia|libnvidia|cuda)' | awk '{print $2}' | tr '\n' ' ')
if [ -n "$NVIDIA_PKGS" ]; then
    apt-get purge -y $NVIDIA_PKGS 2>/dev/null || dpkg --purge --force-depends $NVIDIA_PKGS
fi
rm -f /etc/apt/sources.list.d/cuda-*.list /etc/apt/sources.list.d/nvidia*.list
rm -f /usr/share/keyrings/cuda-*.gpg /usr/share/keyrings/nvidia*.gpg
rm -f /etc/dracut.conf.d/nvidia.conf /etc/dracut.conf.d/blacklist-nouveau.conf
rm -f /etc/apt/apt.conf.d/99nvidia-sha1-exception
apt-get update -q || true
"#;

/// Welcome's install_nvidia_cuda for the 580 and 620 branches. 620 follows
/// NVIDIA's scheme for the debian13 repository: the pinning package, then
/// the nvidia-open meta package (there is no nvidia-open-620).
fn nvidia_install_script(branch: &str) -> String {
    let (repo_setup, install_driver, repo_cleanup) = if branch == "580" {
        (
            r#"echo "deb [trusted=yes] https://developer.download.nvidia.com/compute/cuda/repos/debian12/x86_64/ /" \
    > /etc/apt/sources.list.d/cuda-debian12-x86_64.list"#
                .to_string(),
            "apt-get install -y nvidia-driver-pinning-580\napt-get install -y --allow-downgrades cuda-drivers-580".to_string(),
            "rm -f /etc/apt/sources.list.d/cuda-debian12-x86_64.list\napt-get update -q".to_string(),
        )
    } else {
        (
            r#"command -v wget >/dev/null 2>&1 || apt-get install -y wget
TEMP_DIR=$(mktemp -d)
wget -q -O "$TEMP_DIR/cuda-keyring.deb" \
    https://developer.download.nvidia.com/compute/cuda/repos/debian13/x86_64/cuda-keyring_1.1-1_all.deb
if [ ! -s "$TEMP_DIR/cuda-keyring.deb" ]; then
    echo "ERROR: Failed to download cuda-keyring."
    rm -rf "$TEMP_DIR"
    exit 1
fi
dpkg -i "$TEMP_DIR/cuda-keyring.deb"
rm -rf "$TEMP_DIR""#
                .to_string(),
            format!("apt-get install -y nvidia-driver-pinning-{branch}\napt-get install -y nvidia-open"),
            "apt-get purge -y cuda-keyring\nrm -f /etc/apt/sources.list.d/cuda-debian13-x86_64.list\napt-get update -q".to_string(),
        )
    };
    let patch = nvidia_dkms_patch_commands();
    let llvm = LLVM_DKMS_TOOLCHAIN;
    format!(
        r#"set -e
echo "=== NVIDIA {branch}, kernel $KVER ==="
for entry in $(dkms status 2>/dev/null | grep -i nvidia | awk -F'[:,]' '{{print $1"/"$2}}' | tr -d ' '); do
    dkms remove --force "$entry" --all 2>/dev/null || true
done
rm -rf /var/lib/dkms/nvidia*
rm -f /etc/dracut.conf.d/nvidia.conf /etc/dracut.conf.d/blacklist-nouveau.conf
apt-get purge -y 'nvidia*' 'cuda*' 'libnvidia*' 2>/dev/null || true
apt-get -f install -y 2>/dev/null || true
apt-get install -y dkms build-essential
apt-get install -y linux-headers-$KVER || true
{llvm}
if [ ! -e "/lib/modules/$KVER/build/Makefile" ]; then
    apt-get install -y --reinstall linux-headers-$KVER || true
fi
if [ ! -e "/lib/modules/$KVER/build/Makefile" ]; then
    echo "ERROR: no usable kernel build tree for $KVER."
    exit 1
fi
{repo_setup}
apt-get update
{install_driver} || echo "First attempt failed (the DKMS build runs before the patch); patching the sources and retrying..."
{patch}
dpkg --configure -a
{install_driver}
{repo_cleanup}
dkms autoinstall -k "$KVER" || true
if ! dkms status | grep -qi "nvidia.*installed"; then
    echo "ERROR: the NVIDIA DKMS module did not build for $KVER."
    exit 1
fi
mkdir -p /etc/dracut.conf.d
echo 'omit_drivers+=" nouveau "' > /etc/dracut.conf.d/blacklist-nouveau.conf
echo "NVIDIA driver {branch} installed."
"#
    )
}

/// Welcome's on-demand (render offload) mode for hybrid laptops: the
/// integrated GPU drives the desktop and prime-run sends an app to NVIDIA.
const PRIME_RUN: &str = r#"cat > /usr/local/bin/prime-run << 'PRIMERUN'
#!/bin/bash
export __NV_PRIME_RENDER_OFFLOAD=1
export __NV_PRIME_RENDER_OFFLOAD_PROVIDER=NVIDIA-G0
export __GLX_VENDOR_LIBRARY_NAME=nvidia
export __VK_LAYER_NV_optimus=NVIDIA_only
export __EGL_VENDOR_LIBRARY_FILENAMES=/usr/share/glvnd/egl_vendor.d/10_nvidia.json
export GBM_BACKEND=nvidia-drm
exec "$@"
PRIMERUN
chmod +x /usr/local/bin/prime-run
mkdir -p /usr/share/applications
cat > /usr/share/applications/prime-run.desktop << 'DESKTOP'
[Desktop Entry]
Name=Run with NVIDIA GPU
Comment=Run application using the NVIDIA GPU
Exec=prime-run %f
Icon=nvidia
Terminal=false
Type=Application
NoDisplay=true
DESKTOP
"#;

/// Welcome's uninstall_vbox_guest.
const VBOX_UNINSTALL: &str = r#"UNINSTALLER=$(ls -d /opt/VBoxGuestAdditions-*/uninstall.sh 2>/dev/null | head -1)
if [ -n "$UNINSTALLER" ] && [ -x "$UNINSTALLER" ]; then
    "$UNINSTALLER" || true
fi
apt-get purge -y virtualbox-guest-utils virtualbox-guest-x11 virtualbox-guest-dkms 2>/dev/null || true
"#;

/// Where the bundled VirtualBox Guest Additions are: next to the binary in
/// development, in the package's resources once installed.
fn vbox_run() -> Option<String> {
    let exe = std::env::current_exe().ok()?;
    let dir = exe.parent()?.to_path_buf();
    [
        dir.join("resources/vbox/VBoxLinuxAdditions.run"),
        Path::new("/usr/lib/soplos-linux-installer/resources/vbox/VBoxLinuxAdditions.run").to_path_buf(),
        Path::new(env!("CARGO_MANIFEST_DIR")).join("resources/vbox/VBoxLinuxAdditions.run"),
    ]
    .into_iter()
    .find(|p| p.exists())
    .map(|p| p.to_string_lossy().into_owned())
}

/// Runs a script inside the target with $KVER set. Errors are returned, not
/// raised: a driver must not stop the installation.
fn run_script(ctx: &mut Ctx, kver: &str, name: &str, body: &str) -> Result<(), String> {
    ctx.log(&format!("--- {name} ---"));
    let script = format!("export KVER={kver}\n{body}");
    ctx.run("chroot", &[TARGET, "/bin/bash", "-s"], Some(Stdin::Text(script))).map(|_| ())
}

/// The kernel of the target: the newest in /lib/modules.
fn target_kernel(ctx: &Ctx) -> String {
    if ctx.dry_run {
        return "<kernel version>".into();
    }
    let mut versions: Vec<String> = std::fs::read_dir(format!("{TARGET}/lib/modules"))
        .map(|d| d.flatten().filter_map(|e| e.file_name().into_string().ok()).collect())
        .unwrap_or_default();
    versions.sort();
    versions.pop().unwrap_or_default()
}

/// The scan: what the hardware needs and what it does not, read once before
/// the kernel step.
pub fn scan(ctx: &mut Ctx) -> DriverPlan {
    let lspci = ctx.read("lspci", &[]).unwrap_or_default();
    let lsusb = ctx.read("lsusb", &[]).unwrap_or_default();
    let installed: Vec<String> = ctx
        .read("chroot", &[TARGET, "dpkg", "-l"])
        .unwrap_or_default()
        .lines()
        .filter(|l| l.starts_with("ii"))
        .filter_map(|l| l.split_whitespace().nth(1))
        .map(|s| s.split(':').next().unwrap_or(s).to_string())
        .collect();
    let vm = detect_vm(ctx);
    let vbox_enabled = Path::new(&format!("{TARGET}/etc/systemd/system/multi-user.target.wants/vboxadd.service")).exists();
    let vendor = read_dmi("sys_vendor").to_lowercase();
    let mac = (vendor.starts_with("apple"), read_dmi("product_name"), facetime_camera_present());
    let plan = plan(&lspci, &lsusb, &installed, vm, vbox_enabled, mac);
    for line in &plan.summary {
        ctx.log(line);
    }
    plan
}

/// What the hardware does not need. It runs before the kernel step, so the
/// new kernel does not build DKMS modules (VirtualBox Guest Additions,
/// NVIDIA) that are about to go.
pub fn remove(ctx: &mut Ctx, plan: &DriverPlan) {
    let kver = target_kernel(ctx);
    if plan.purge_nvidia {
        let _ = run_script(ctx, &kver, "NVIDIA/CUDA without an NVIDIA GPU", NVIDIA_UNINSTALL);
    }
    if plan.remove_vbox {
        let _ = run_script(ctx, &kver, "VirtualBox Guest Additions outside VirtualBox", VBOX_UNINSTALL);
    }
    if !plan.purge.is_empty() {
        let mut args = vec![TARGET, "apt-get", "-y", "purge"];
        args.extend(plan.purge.iter().map(String::as_str));
        if let Err(e) = ctx.run_progress("chroot", &args, 0, 0) {
            ctx.log(&format!("WARNING: {e}"));
        }
    }
}

/// What is missing, built for the new kernel. Returns whether the NVIDIA
/// driver is installed, so the boot loader step adds nvidia-drm.modeset=1.
pub fn install(ctx: &mut Ctx, plan: &DriverPlan) -> bool {
    let kver = target_kernel(ctx);

    // What is missing. One by one if the batch fails, so one package that
    // the repositories lack does not cost the others.
    if !plan.install.is_empty() {
        let mut args = vec![TARGET, "apt-get", "-y", "install"];
        args.extend(plan.install.iter().map(String::as_str));
        if ctx.run_progress("chroot", &args, 0, 0).is_err() {
            for package in &plan.install {
                if let Err(e) = ctx.run_progress("chroot", &[TARGET, "apt-get", "-y", "install", package], 0, 0) {
                    ctx.log(&format!("WARNING: {package}: {e}"));
                }
            }
        }
    }
    if plan.mac_tlp {
        // As Welcome: --no-remove, TLP never removes power-profiles-daemon behind the user's back.
        let _ = ctx.run_progress("chroot", &[TARGET, "apt-get", "-y", "install", "--no-remove", "tlp", "tlp-rdw"], 0, 0);
    }

    // NVIDIA.
    let mut nvidia = false;
    if let Some(branch) = &plan.nvidia {
        match run_script(ctx, &kver, &format!("NVIDIA {branch}"), &nvidia_install_script(branch)) {
            Ok(()) => {
                nvidia = true;
                if plan.hybrid {
                    let _ = run_script(ctx, &kver, "PRIME render offload (prime-run)", PRIME_RUN);
                }
            }
            Err(e) => {
                // Back to nouveau: a half-installed driver would leave no
                // graphics at all, since it blacklists nouveau.
                ctx.log(&format!("WARNING: the NVIDIA driver was not installed ({e}); nouveau stays. Soplos Welcome can install it later."));
                let _ = run_script(ctx, &kver, "NVIDIA cleanup", NVIDIA_UNINSTALL);
            }
        }
    }

    // VirtualBox Guest Additions, from the bundled installer.
    if plan.vbox_additions {
        match vbox_run() {
            Some(run) => {
                let inside = "/tmp/VBoxLinuxAdditions.run";
                let copied = ctx.run("cp", &[&run, &format!("{TARGET}{inside}")], None).is_ok();
                if copied {
                    let body = format!(
                        "apt-get install -y build-essential dkms linux-headers-$KVER\n{LLVM_DKMS_TOOLCHAIN}\nKERN_VER=$KVER sh {inside} --nox11 || true\nrm -f {inside}\n"
                    );
                    if let Err(e) = run_script(ctx, &kver, "VirtualBox Guest Additions", &body) {
                        ctx.log(&format!("WARNING: {e}"));
                    }
                }
            }
            None => ctx.log("WARNING: the VirtualBox Guest Additions installer is not in the package."),
        }
    }

    if plan.facetimehd {
        ctx.log("FaceTime HD camera: its driver is built from sources; Soplos Welcome installs it after the first start.");
    }
    nvidia
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nvidia_branches() {
        assert_eq!(recommend_nvidia("GeForce RTX 3060"), "nvidia-620");
        assert_eq!(recommend_nvidia("GeForce GTX 1650"), "nvidia-620");
        assert_eq!(recommend_nvidia("GeForce GTX 960M"), "nvidia-580");
        assert_eq!(recommend_nvidia("GeForce GTX 1060 6GB"), "nvidia-580");
        assert_eq!(recommend_nvidia("GeForce GTX 750 Ti"), "nouveau");
        assert_eq!(recommend_nvidia("GeForce GT 630"), "nouveau");
        assert_eq!(recommend_nvidia("Quadro P2000"), "nouveau");
        // "t550" is a Turing card; inside "gtx550" it is not.
        assert_eq!(recommend_nvidia("GeForce gtx550"), "nouveau");
    }

    #[test]
    fn a_hybrid_laptop_on_real_hardware() {
        let lspci = "00:02.0 VGA compatible controller: Intel Corporation HD Graphics 4600 (rev 06)\n\
                     01:00.0 3D controller: NVIDIA Corporation GM107M [GeForce GTX 960M] (rev a2)\n\
                     00:1b.0 Audio device: Intel Corporation 8 Series/C220 Series High Definition Audio Controller\n\
                     03:00.0 Network controller: Intel Corporation Wireless 7260 (rev 73)\n";
        let installed: Vec<String> = ["alsa-utils", "pipewire", "open-vm-tools", "spice-vdagent", "mesa-vulkan-drivers"].iter().map(|s| s.to_string()).collect();
        let p = plan(lspci, "", &installed, Vm::default(), false, (false, String::new(), false));
        assert_eq!(p.nvidia.as_deref(), Some("580"));
        assert!(p.hybrid);
        assert!(p.install.contains(&"intel-media-va-driver".to_string()));
        assert!(p.install.contains(&"firmware-iwlwifi".to_string()));
        assert!(!p.install.contains(&"mesa-vulkan-drivers".to_string()), "already installed");
        assert!(!p.install.contains(&"pulseaudio".to_string()), "PipeWire is a sound server");
        assert_eq!(p.purge, vec!["open-vm-tools".to_string(), "spice-vdagent".to_string()], "VM tools on real hardware");
    }

    #[test]
    fn a_vmware_guest() {
        let lspci = "00:0f.0 VGA compatible controller: VMware SVGA II Adapter\n";
        let installed: Vec<String> = ["qemu-guest-agent", "nvidia-driver-pinning-580"].iter().map(|s| s.to_string()).collect();
        let vm = Vm { is_vm: true, kind: "VMware".into(), required: vec!["open-vm-tools".into(), "open-vm-tools-desktop".into()] };
        let p = plan(lspci, "", &installed, vm, false, (false, String::new(), false));
        assert!(p.install.contains(&"open-vm-tools-desktop".to_string()));
        assert!(p.purge.contains(&"qemu-guest-agent".to_string()), "tools of another hypervisor");
        assert!(p.purge_nvidia, "NVIDIA packages with no NVIDIA GPU");
        assert!(p.nvidia.is_none());
    }
}

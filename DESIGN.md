# Soplos Linux Installer (SLI) 3.0 - Design

Status: draft for review. Nothing is implemented yet.
Target: the Soplos Linux ISOs 2027.01 (2027.02 at the latest), for Tyron (Xfce),
Tyson (Plasma) and Boro (GNOME).

SLI replaces Calamares. Everything Calamares does today must keep working (see
section 12), and SLI adds what Calamares cannot do: the choice of boot loader,
and hardware detection with automatic driver installation and removal, as
Soplos Welcome does.

---

## 1. Technology and packaging

- The same stack as the other 3.0 programs: Tauri 2, a Rust backend, Svelte 4,
  Tailwind CSS and svelte-i18n. The UI is rendered by the system WebKitGTK.
- 8 languages: en, es, de, fr, it, pt, ro, ru.
- Package `soplos-linux-installer`, binary `soplos-linux-installer`, identifier
  `org.soplos.linuxinstaller`. The first version is `3.0.0-alpha`.
- The same window layout, theme handling and dialogs as the rest of the 3.0
  family.
- It runs only in the live session. It is removed from the installed system.

## 2. Launch and privileges

- Soplos Welcome Live starts it through its `installer.json`, which goes from
  Calamares to SLI:
  `{"name": "SLI", "paths": ["/usr/bin/soplos-linux-installer"], "needs_root": false}`.
- NumLock (Tyron): Welcome Live's tools leave the mark
  `/etc/soplos-linux-installer/numlock-off` in the live session to turn it
  off; the last step of the helper then comments out
  `greeter-setup-script=/usr/bin/numlockx on` in lightdm.conf and purges
  numlockx, as the Tyron Calamares did.
- The window runs as the live user and never as root.
- The disk work is done by the same binary in helper mode
  (`soplos-linux-installer --helper`), started with `sudo -n`. The live user has
  passwordless sudo, as Welcome Live already uses.
- The window sends the helper the complete installation plan as one JSON
  document. The helper reports progress as JSON lines (step, percentage,
  message, log line) and a final result. It accepts nothing else.
- Everything the helper runs is logged to `/var/log/soplos-linux-installer.log`,
  which is copied to the installed system at the end.

## 3. Requirements check (first screen)

| Check | Boro / Tyron | Tyson | Blocks the installation |
|---|---|---|---|
| Disk | 18 GB | 28 GB | yes |
| RAM | 1.8 GB | 3.6 GB | yes |
| Internet | required | required | yes (drivers and Soplos Welcome are downloaded) |
| Power | on AC when there is a battery | same | no, warning only |

The values come from the edition file (section 10).

## 4. Screens

1. **Welcome and language.** It shows the requirements check.
2. **Location:** time zone (geolocated, as Calamares does), system locale and
   keyboard layout with a test field.
3. **Boot loader.**
   - The firmware is detected: UEFI or BIOS.
   - UEFI offers GRUB, systemd-boot and Limine. BIOS offers GRUB and Limine.
     GRUB is preselected.
   - All three allow Secure Boot: GRUB and systemd-boot behind shim with the
     signatures of Debian (systemd-boot needs shim-signed and the signed
     systemd-boot package, to verify), Limine signed with the user's MOK
     from Soplos Kernel Installer. No warning against any of them.
4. **Kernel.**
   - The x86-64 level of the processor (section 7b) and the Soplos kernels
     the repository has for it, read as Soplos Kernel Installer reads them:
     stock (preselected, recommended), BORE + NTsync, Zen, RT, X3D and any
     other the repository adds, each in its latest or its LTS branch. The
     level is not a choice: a kernel for a higher level would not boot.
   - The kind of installation, each adding to the one before:
     **minimal** copies the image as it is (its kernel, no driver changes);
     **standard** removes the drivers the hardware does not need, installs
     the kernel chosen and the missing drivers; **complete** (preselected)
     also runs `apt-get full-upgrade` after the new kernel and before the
     drivers. With minimal the kernel cards are disabled.
   - Without the repository the stock kernel is still tried during the
     installation, and the kernel of the image stays if it fails.
5. **Disk.**
   - Pick the disk.
   - Pick the mode: erase the disk, install alongside another system, or
     manual.
   - Options: root file system (btrfs by default, ext4, xfs), encryption
     (LUKS) and swap (none / swap / swap for hibernation; "swap" is the
     default).
   - A live preview of the partitions that will be created.
6. **User:**
   - full name, login name, computer name and password;
   - the choice of using the same password for root or a different one, as
     Calamares does.
7. **Summary** of everything that will be done, plus the last confirmation
   before any disk is touched.
8. **Installation:** progress bar, current step, and the motion graphics
   (section 9). A "Show details" toggle reveals the log.
9. **Finished:** restart now (checked by default) or keep using the live
   session.

Incompatible options are blocked in the UI, not just warned about. Examples:
systemd-boot on BIOS, or a manual layout without the partitions the chosen boot
loader needs.

## 5. Partitioning

### 5.1 Automatic layouts ("erase the disk")

Always GPT. The swap partition goes at the end of the disk.

| Firmware | Boot loader | Partitions, in order |
|---|---|---|
| UEFI | GRUB | ESP FAT32 1 GiB `/boot/efi` - `/boot` ext4 2 GiB - root - swap |
| UEFI | systemd-boot | ESP FAT32 2 GiB `/boot/efi` - `/boot` ext4 2 GiB - root - swap |
| UEFI | Limine | ESP FAT32 2 GiB `/boot/efi` - `/boot` ext4 2 GiB - root - swap |
| BIOS | GRUB | bios_grub 1 MiB - `/boot` ext4 2 GiB - root - swap |
| BIOS | Limine | bios_grub 1 MiB - FAT32 2 GiB `/boot/limine` - `/boot` ext4 2 GiB - root - swap |

- **Root on btrfs:** subvolumes `@` (mounted at `/`) and `@home` (mounted at
  `/home`), the same as Calamares today and as Timeshift requires.
- **Swap:** 8 GiB, or the RAM plus its square root when "swap for
  hibernation" is chosen (at least 9 GiB, so it is never the same as the
  plain swap). zRAM stays as it is in the image.
- **Why the ESP sizes differ:** systemd-boot and Limine keep a copy of every
  kernel and initrd on the ESP (about 160 MB per kernel), so their ESP is as
  big as `/boot`.

### 5.2 Manual

The user assigns mount points and file systems to existing or new partitions,
for example the user's own layout: `/boot/efi`, `/boot`, `/` on btrfs, `/home`
on xfs. If `/home` is on its own partition, the root btrfs has only `@`.

Rules checked before going on, with the button blocked until they are met:

- UEFI: there is a FAT32 ESP with the esp flag mounted at `/boot/efi`.
- systemd-boot or Limine: the ESP is at least as large as `/boot`, or as the
  space the kernels need when `/boot` is not separate.
- BIOS on GPT: there is a bios_grub partition (at least 1 MiB).
- Limine on BIOS: there is a FAT partition mounted at `/boot/limine`.
- `/` exists and is big enough for the edition.

### 5.3 Alongside another system

- It shrinks one existing partition chosen by the user (NTFS, ext4 or btrfs)
  and installs into the freed space. The size is chosen with a slider.
- Each file system is checked before it is resized:
  - NTFS: `ntfsfix`/`ntfsresize --check`. Windows hibernation or Fast Startup
    blocks the resize, with an explanation.
  - ext4: `e2fsck`.
- The existing ESP is reused.
  - With systemd-boot or Limine, an ESP smaller than needed (Windows creates
    100 to 260 MiB) gets an extra FAT32 partition: an XBOOTLDR mounted at
    `/boot` for systemd-boot, or the FAT that Limine reads.
- The new system's EFI folder is `\EFI\<edition>`. If it already exists, the
  user chooses to replace it or to use `<edition>-2`.
- The other systems appear in the boot menu: os-prober for GRUB, and the ESP
  scan of limine-soplos for Limine. systemd-boot detects Windows on the same
  ESP by itself.

### 5.4 Encryption (LUKS)

- LUKS2 on the root partition, and on `/home` if it is separate. `/boot` and
  the ESP stay unencrypted, so all three boot loaders work the same way.
- One passphrase at boot, asked by the initrd: dracut `crypt` module, with
  `/etc/crypttab` written by SLI.
- Encryption is a checkbox on the disk screen, unchecked by default. Without
  it nothing is encrypted and no passphrase is ever asked at boot.
- The swap follows the two choices the user already makes, with no extra
  technical question:

  | User choice | Swap |
  |---|---|
  | No encryption | Plain swap partition |
  | Encryption + no swap | No swap |
  | Encryption + swap | Swap encrypted with a random key at every boot (no prompt, no hibernation) |
  | Encryption + swap for hibernation | Its own LUKS2 container with the same passphrase as the root partition: one prompt, unlocked by the initrd, hibernation works |

- No key file is stored on the unencrypted `/boot`. To be verified in a VM.

## 6. Installation steps (helper)

Each step reports its progress. A failure stops the installation, keeps the
log, and leaves the disk untouched if partitioning had not started yet.

1. **Partition and format.** Encryption is set up here when chosen.
2. **Mount the target** at `/target` (subvolumes `@` and `@home`, then the
   other partitions), plus the bind mounts proc, sys, dev, dev/pts, run and
   efivars.
3. **Copy the system:** unpack `filesystem.squashfs` into `/target`. Its path
   is detected at run time (`/run/initramfs/live/live/filesystem.squashfs` or
   the live-boot path).
4. **Base configuration:**
   - machine-id;
   - `/etc/fstab` with the mount options of section 7, and `/etc/crypttab`;
   - time zone and hardware clock;
   - locale and `LANGUAGE`, and the keyboard for the console and for X11;
   - hostname and hosts.
5. **Users:**
   - create the user with the groups sudo, adm, cdrom, audio, video, plugdev,
     users, netdev and lp. These are the groups of the live user in the
     production image, without `disk` and `autologin`. The image has no
     `wheel` group, and Debian's `%sudo ALL=(ALL:ALL) ALL` rule already
     exists, so no sudoers file is added;
   - set the passwords (root the same as the user's, or its own);
   - remove liveuser and its sudo rules.
6. **Display manager** of the edition: remove the live autologin and enable or
   configure the service (section 10).
7. **Packages** (apt in the chroot; the internet is already checked):
   - remove the live packages: live-config, live-config-doc,
     live-config-systemd, calamares-settings-debian, calamares,
     soplos-welcome-live, soplos-linux-installer (Tyson also
     calamares-extensions-data);
   - install `soplos-welcome`. Its first-boot autostart is already in
     `/etc/skel`.
8. **Drivers** (section 8).
9. **Kernel command line and boot loader:**
   - write `/etc/kernel/cmdline` (`root=UUID=... ro rootflags=subvol=@ quiet
     splash`, plus the `cryptdevice` and `resume` parameters when they apply)
     and `/etc/kernel/entry-token` = edition codename;
   - **GRUB:** `grub-install --bootloader-id=<edition>` with the EFI fallback.
     The image's own `/etc/default/grub` is kept, with its theme and
     graphical terminal, as Calamares does (its defaults only apply when the
     file is missing). Only the LUKS and resume parameters are added, and
     `GRUB_DISABLE_OS_PROBER=false` when installing alongside another
     system. Then `update-grub`.
   - **systemd-boot:** install `systemd-boot` (Debian installs it on the ESP
     and keeps the entries); purge GRUB and shim with
     `--allow-remove-essential`, so a later GRUB upgrade cannot take the
     first place in the boot order.
   - **Limine:** install `limine`, `limine-soplos` and `soplos-limine-theme`,
     then `limine-install --force`; purge GRUB and shim as above.
10. **Initramfs:** `update-initramfs -u -k all`. It is dracut's own wrapper, and
    it runs the post-update hooks, so systemd-boot gets the new initrds.
11. **Edition finalization** (section 10) and the common cleanup:
    - copy `/etc/skel` to the user's home and set ownership;
    - create the user's XDG folders;
    - remove `/etc/apt/sources.list`;
    - remove the installer and live leftovers.
12. **Unmount** everything and close LUKS. Copy the log into the target before
    unmounting.

## 7. Mount options (all editions)

| File system | Options |
|---|---|
| ESP (vfat) | `defaults,umask=0077` |
| btrfs | `defaults,noatime,compress=zstd:1`, plus `discard=async` on SSD |
| ext4 | `defaults,noatime` |
| xfs | `defaults,noatime` |
| swap | `sw` |

These are the options Tyron's Calamares uses today, now applied to the three
editions.

## 7b. Kernel

After the packages (`src-tauri/src/helper/kernel.rs`), between the removal
of the drivers the hardware does not need and the installation of the
missing ones, so DKMS builds only what stays: the latest stock kernel of
Soplos for the x86-64 level of the processor, `linux-soplos-v1` to `-v4`,
from the Soplos Kernels repository the image already has enabled. The
level is the one the dynamic linker reports as supported (`ld.so --help`),
as Soplos Kernel Installer shows it; the CPU flags only if that fails. Once it is in, the kernel of the image is purged with its
headers (user decision, 2026-11-13: one kernel only). If the package cannot
be installed, the kernel of the image stays and the installation goes on.

## 8. Hardware detection and drivers

This is a port of Soplos Welcome 3.0 (`hardware_scan.rs`, `drivers.rs`,
`mac.rs`), adapted as little as possible. It runs automatically, with no
questions, during the installation, inside the chroot of the target.

- **Detection** is identical to Welcome:
  - GPUs (NVIDIA branch per model, AMD, Intel, virtual, hybrid);
  - Wi-Fi firmware per vendor;
  - audio, Bluetooth and USB printers;
  - virtual machines;
  - Macs (DMI).
- **NVIDIA:** only the branches 580, 590, 610 and 620 are installed, with
  Welcome's exact procedure:
  - clean the previous NVIDIA packages and DKMS modules;
  - headers and DKMS (plus clang/lld/llvm for kernels built with Clang);
  - official NVIDIA repository and pinning;
  - first attempt, Soplos DKMS patches, retry;
  - remove the repository and run `dkms autoinstall`;
  - nouveau blacklisted in dracut;
  - `nvidia-drm.modeset=1` written where the chosen boot loader reads it
    (`/etc/default/grub` for GRUB, `/etc/kernel/cmdline` for the others).
- Every GPU that would need the 550 branch or older (Kepler, Fermi, older
  cards, Debian's `nvidia-driver`) stays on **nouveau**. The user can change
  it later in Soplos Welcome.
- **Hybrid laptops:** the driver is installed with no forced profile. The
  integrated GPU drives the desktop, NVIDIA renders on demand, and SLI adds the
  `prime-run` helper from Welcome's on-demand mode.
- **AMD, Intel, Wi-Fi, Bluetooth, printers, VM guest tools and Mac items:**
  the same packages Welcome installs. VirtualBox gets its Guest Additions from
  the bundled `.run`.
- **Implemented** in `src-tauri/src/helper/drivers.rs` (scan, plan, scripts)
  and `nvidia_patches.rs` (Welcome's DKMS patches, copied verbatim). It runs
  after the packages and before the boot loader, which writes
  `nvidia-drm.modeset=1` and regenerates the initramfs. A failed NVIDIA
  install is rolled back to nouveau and does not stop the installation.
  Two differences with Welcome: the GTX 16 series is detected as Turing
  (Welcome sends it to nouveau, a bug to fix there), and on a virtual
  machine the guest tools of the other hypervisors are removed too. The
  FaceTime HD camera driver is left to Welcome (it is built from sources).
- **Removal of what the hardware does not need,** as in Welcome:
  - NVIDIA/CUDA without an NVIDIA GPU;
  - AMD/ROCm without an AMD GPU;
  - Broadcom packages without a Broadcom adapter;
  - VM guest tools on real hardware;
  - VirtualBox Guest Additions outside VirtualBox.
- **Chroot adaptations:**
  - the kernel version of the target instead of `$(uname -r)`;
  - `dkms autoinstall -k <version>`;
  - `systemctl --root` or `enable` instead of acting on the live session;
  - `update-initramfs` instead of a direct `dracut --force`.
- **NVIDIA branch selection** (the CUDA debian13 repository, checked on
  2026-11-13):
  - `nvidia-driver-pinning-NNN` only installs
    `/etc/apt/preferences.d/nvidia-driver-pin`. That file pins
    `src:nvidia-open`, `cuda-drivers*`, the graphics drivers and their tools
    to version `NNN*` with priority 1000.
  - The driver is then installed with the generic meta package:
    `apt install nvidia-driver-pinning-620`, then `apt install nvidia-open`.
  - The repository has no package literally named `nvidia-open-NNN`, but
    Welcome installs `nvidia-open-590`/`nvidia-open-610` today. This is to be
    verified on real hardware and fixed in Welcome and SLI alike.
  - 610 and 620 are open kernel modules, which only support Turing and newer
    GPUs.

## 9. Motion graphics during the installation

- Full-window HTML/CSS animation inside SLI, offline (no web fonts or remote
  assets), in the 8 languages, looping while the installation runs.
- Content:
  - Soplos Linux;
  - the three editions (Tyron, Tyson, Boro) and what each offers;
  - the 16 Soplos programs;
  - community, website and donations;
  - final tips, such as Soplos Welcome on first boot.
- It builds on the presentation in `soplos-artwork/presentación`: the same
  style and icons, with texts separated by language as `textos.js` already
  does.
- Its texts are written in Spanish first and then translated. The user
  approves them.

## 10. Per-edition data

One file per edition in `/usr/share/soplos-linux-installer/editions/<codename>.json`,
so no edition-specific logic is hard-coded.

| | Boro | Tyron | Tyson |
|---|---|---|---|
| Display manager | gdm3 | lightdm | plasmalogin |
| Autologin removal | GDM `daemon.conf` (AutomaticLogin, TimedLogin) | LightDM `autologin-user`, `/etc/sudoers.d/restart-lightdm` | none |
| Service action | gdm3 | lightdm | enable plasmalogin.service |
| Extra finalization | none | LightDM background (`configure-lightdm.sh`) | greeter language (`fix-plasmalogin-locale.sh`) |
| Requirements | 18 GB / 1.8 GB | 18 GB / 1.8 GB | 28 GB / 3.6 GB |

Not ported, because they are obsolete: `clean-modules.sh` (NVIDIA drivers are
no longer preinstalled), Tyron's `remove-apps.sh`, and the office-suite
chooser.

## 11. Testing

- Hypervisors: virt-manager, Proxmox, VMware and VirtualBox. Real hardware:
  Bella (Intel + GTX 960M, hybrid).
- Matrix:
  - UEFI with GRUB, systemd-boot and Limine; BIOS with GRUB and Limine;
  - each with erase, manual, alongside (Windows), and LUKS;
  - in the three editions.
- After each installation, check:
  - the system boots;
  - a kernel update works from the terminal;
  - the initrd regenerates with `update-initramfs`;
  - the drivers are right for the hardware;
  - Timeshift sees `@` and `@home`.

## 12. Parity checklist with the current Calamares

The current Calamares configuration of each edition is in
`scripts_{boro,tyron,tyson}/`.

- Requirements check; geolocated time zone; locale and keyboard.
- Erase, alongside and manual; btrfs (default), ext4 and xfs; swap choices;
  LUKS.
- Subvolumes `@` and `@home`.
- User with full name, login, computer name, password, and the choice of root
  password.
- Display manager setup and liveuser removal.
- Live packages removed and `soplos-welcome` installed.
- Initramfs with dracut. Boot loader installed with an EFI fallback and the
  edition's EFI id.
- `/etc/default/grub` defaults and Plymouth (quiet splash).
- The cleanup scripts of each edition.
- Restart at the end.

## 13. Open points

1. NVIDIA: test the pinning plus `nvidia-open` install on real hardware
   (section 8). Turing and newer GPUs get the 620 branch (decided on
   2026-11-13, since it covers the same GPUs as 610).
2. The texts of the motion graphics (section 9). They are written once the
   program skeleton exists, so the available space is known.

Settled on 2026-11-13: the ESP with GRUB is 1 GiB, the size Calamares uses.
Swap with LUKS shares the root passphrase (section 5.4).

## 14. Later, outside SLI

- The default kernel after installing a new one (ordering in GRUB and Limine).
- Steps 4 of the bootloader plan: the 3.0 programs detect the boot loader
  instead of assuming GRUB, and use `update-initramfs` instead of `dracut`.
- The `\EFI\BOOT` fallback and other cosmetic details of Limine.

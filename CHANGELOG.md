# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/).

## [3.0.0-3-beta] - 2026-11-15

### Fixed
- Installing alongside another Linux made a second swap partition even when
  the disk already had one, so the existing swap was left unused. A machine
  has one swap for all its systems now: when the disk already has a swap,
  Soplos always shares it as it is (written in its fstab, not formatted,
  and not encrypted when Soplos is), whatever the swap choice, and never
  makes another one. The disk step shows "The existing swap, shared" with
  its size instead of the swap choice, and so does the summary. With no
  swap on the disk the choice is the same as before.
- A swap partition marked as "use" (alongside, or in manual partitioning)
  was formatted again with `mkswap`, which gave it a new UUID and left the
  other system without swap. An existing swap is now used without
  formatting it.

## [3.0.0-2-beta] - 2026-11-14

### Fixed
- The folders of the new user (Documents, Downloads...) were made in
  English, because the helper runs every command with `LC_ALL=C`, which
  outweighed the language passed to `xdg-user-dirs-update`. At the first
  login the desktop asked to rename them to the system's language, and the
  bookmarks pointed to the English names. They are made now as the
  `setup-desktop.sh` of Calamares made them, with `su -` (a clean login
  environment that reads `/etc/default/locale`), so they are in the
  system's language from the start. A failure is written to the log.

## [3.0.0-1-beta] - 2026-11-14

### Added
- NumLock off on the installed system when it was turned off in the tools
  of Soplos Welcome Live (Tyron): the mark
  `/etc/soplos-linux-installer/numlock-off` of the live session makes the
  last step comment out the numlockx line of the greeter and purge
  numlockx, as the Tyron Calamares did.

## [3.0.0-beta] - 2026-11-14

### Added
- Kernel step, after the boot loader: the kind of installation, minimal
  (the image as it is), standard (drivers and the kernel chosen) or
  complete (and the system brought up to date with `apt-get full-upgrade`,
  preselected); and the Soplos kernels the repository has for the level of
  the processor, read as Soplos Kernel Installer reads them: stock (the
  recommended one), BORE + NTsync, Zen, RT and X3D, in their latest or
  their LTS branch. If the kernel chosen cannot be installed, the stock one
  is tried. The summary shows both. Alt+1 to Alt+7 open the steps.
- The latest stock Soplos kernel for the x86-64 level of the processor
  (`linux-soplos-v1` to `-v4`, detected as Soplos Kernel Installer does),
  installed before the drivers so they are built for it; the kernel of the
  image is then removed. If it cannot be installed, the kernel of the image
  stays.
- Drivers, with the hardware scan and the driver flows of Soplos Welcome:
  what the hardware does not need is removed (NVIDIA/CUDA without an NVIDIA
  GPU, AMD/ROCm without an AMD GPU, Broadcom without a Broadcom adapter,
  guest tools of virtual machines on real hardware or of another
  hypervisor) and what is missing is installed (AMD and Intel graphics,
  Wi-Fi firmware, audio, Bluetooth, printers, the guest tools of the
  virtual machine, VirtualBox Guest Additions, Mac fan and battery tools).
  NVIDIA: the 620 branch for Turing and newer, 580 for Maxwell and Pascal,
  with the Soplos DKMS patches; every other card stays on nouveau. Hybrid
  laptops get `prime-run`. A driver that fails does not stop the
  installation.

### Fixed
- GRUB: the `/etc/default/grub` of the image is kept, so the Soplos theme
  shows; only the kernel parameters the installation needs are added.
- The drivers the hardware does not need (VirtualBox Guest Additions,
  NVIDIA, the tools of another hypervisor) are removed before the new
  kernel is installed, so it does not build their DKMS modules for nothing.
- The x86-64 level of the processor is read from the dynamic linker, as
  Soplos Kernel Installer shows it, and no longer from the flags alone: an
  Intel N150 under Proxmox got the v2 kernel instead of v3.

## [3.0.0-alpha] - 2026-11-14

First version of the installer that replaces Calamares. Every screen is
written and the helper carries out the whole installation. The design is
in `DESIGN.md`.

### Added
- The window of the 3.0 family: no system frame, a light and a dark theme
  that follow the desktop, the About dialog (F1), 8 languages (English,
  Spanish, German, French, Italian, Portuguese, Romanian and Russian), and
  the steps in a sidebar; only the steps already reached can be opened
  again, and none once the installation has started.
- Welcome: the language of the installer and the requirements of the
  edition (disk, memory and internet stop the installation, the battery
  only warns), read from the data file of the edition.
- Location: a world map drawn from data built into the program, with a dot
  per time zone, zoom on the place chosen, and the time zone, language,
  formats and keyboard proposed from the place found by geolocation. The
  layout chosen is put on the live session to try it in a test field.
- Boot loader: GRUB, systemd-boot or Limine on UEFI, GRUB or Limine on BIOS,
  with how each one keeps Secure Boot.
- Disk: erase the disk, install alongside another system (the partition to
  shrink is measured and the space is split with a slider) or manual
  partitioning (GPT or MBR table, uses, file systems, new partitions at the
  start or the end of the free space, and the checks of each boot loader).
  One drawing of the disk shows it as it is, then as it will be.
  Encryption and swap (none, 8 GiB, or for hibernation).
- User: full name, username and computer name proposed from it, password
  with a strength guide, and root with the same password or its own.
- Summary: everything chosen, the disk as it will be, and what cannot be
  undone in plain words.
- Installation: a presentation of the edition, Soplos Linux, the three
  editions and the programs, in 8 languages, while the installation runs;
  the log under "Show details"; the progress bar of the other 3.0 programs.
- Finished: restarts the computer.
- The helper (`--helper`): partitions with sfdisk, shrinks NTFS, ext4 and
  btrfs, formats, LUKS2, btrfs subvolumes `@` and `@home`, copy of the
  system from the live image, fstab and crypttab, time zone, language,
  keyboard, computer name, the user, the display manager of each edition,
  the live packages out and Soplos Welcome in, GRUB, systemd-boot or Limine,
  the initramfs, and the live leftovers removed. With `--dry-run`
  (`SLI_DRY_RUN=1`) it only lists the commands.
- What differs between editions is in their data files
  (`/usr/share/soplos-linux-installer/editions/`).
- Keyboard: hold Alt to see the letter of each button, Alt and that letter
  press it; Alt+arrows go back and forth. Listed in the README.

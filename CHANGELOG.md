# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/).

## [3.0.0-alpha] - 2026-11-14

First version of the installer that replaces Calamares. Every screen is
written and the helper carries out the whole installation, drivers
included. The design is in `DESIGN.md`.

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
- What differs between editions is in their data files
  (`/usr/share/soplos-linux-installer/editions/`).
- Keyboard: hold Alt to see the letter of each button, Alt and that letter
  press it; Alt+arrows go back and forth. Listed in the README.

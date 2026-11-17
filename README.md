# Soplos Linux Installer 3.0

![Version](https://img.shields.io/badge/version-3.0.0-blue)
![License](https://img.shields.io/badge/license-GPL--3.0--or--later-green)
![Platform](https://img.shields.io/badge/platform-Linux-lightgrey)
![Tauri](https://img.shields.io/badge/Tauri-2-orange)
![Rust](https://img.shields.io/badge/Rust-1.80+-orange)
![Svelte](https://img.shields.io/badge/Svelte-4-red)

The installer of Soplos Linux, for Tyron (Xfce), Tyson (Plasma) and Boro
(GNOME). It replaces Calamares.

| ![Welcome](src-tauri/resources/screenshots/screenshot01.png) Welcome: the language of the installer and the requirements of the edition | ![Location](src-tauri/resources/screenshots/screenshot02.png) Location: the map, the language, the formats and the keyboard |
|:---:|:---:|
| ![Boot loader](src-tauri/resources/screenshots/screenshot03.png) Boot loader: GRUB, systemd-boot or Limine | ![Kernel](src-tauri/resources/screenshots/screenshot04.png) Kernel: the kind of installation and the Soplos kernel for the processor |
| ![Disk](src-tauri/resources/screenshots/screenshot05.png) Disk: the disk as it will be, before anything is touched | ![User](src-tauri/resources/screenshots/screenshot06.png) User: the account, the computer name and the passwords |
| ![Summary](src-tauri/resources/screenshots/screenshot07.png) Summary: everything chosen, and what will be done to the disk | ![Editions](src-tauri/resources/screenshots/screenshot08.png) Installation: the presentation of the three editions |
| ![Programs](src-tauri/resources/screenshots/screenshot09.png) Installation: the programs of Soplos, one by one | |

## Status

Beta. Every step of the window is written, and the helper installs the
system: partitions, encryption, the copy of the system, its configuration,
the user, the packages, the drivers and the boot loader. The complete
design is in [DESIGN.md](DESIGN.md).

To check a plan without touching any disk, start it as a dry run: the
helper lists the commands it would run.

```bash
SLI_DRY_RUN=1 npm run tauri dev
```

## Keyboard

The installer can be used without the mouse, as Calamares.

- **Alt + the underlined letter** presses a button. Holding Alt underlines
  one letter of each button; the letters come from the labels in the
  language chosen (for example *Next* and *Back*, or *Siguiente* and *Atrás*).
- **Alt+Right / Alt+Left:** next / back.
- **Alt+1 … Alt+7:** a step already reached.
- **Tab / Shift+Tab** move between the fields; **Space** or **Enter** press
  the button or option in focus.
- **Enter / Escape** accept or cancel a dialog.
- **F1:** about. **Ctrl+Q:** quit (not while the installation runs).

## Features

- **Boot loader of your choice:** GRUB, systemd-boot or Limine on UEFI, GRUB
  or Limine on BIOS, with the partitions each one needs.
- **Kind of installation and kernel:** minimal (the image as it is),
  standard (drivers and the kernel chosen) or complete (and the system
  brought up to date); the Soplos kernels for the level of the processor:
  stock, BORE + NTsync, Zen, RT or X3D, latest or LTS.
- **Disk:** erase the disk, install alongside another system, or manual
  partitioning; btrfs (with the `@` and `@home` subvolumes Timeshift uses),
  ext4 or xfs; optional encryption; swap. Manual partitioning offers the
  mount points of Calamares (`/`, `/boot`, `/home`, `/opt`, `/srv`, `/usr`,
  `/var`) and mount points of data of your own, with btrfs, ext4, xfs, F2FS
  and JFS for the system and also exFAT, NTFS and FAT32 for data. A partition
  of data can be kept as it is and is written in the fstab with `nofail`.
- **Drivers for the hardware:** the hardware is detected as Soplos Welcome
  does, and during the installation the missing drivers are installed and
  the ones that do not apply are removed.
- **A presentation of Soplos Linux** while the system is installed.
- **Eight languages:** English, Spanish, German, French, Italian, Portuguese,
  Romanian and Russian.

## Architecture

The same stack as the other 3.0 programs: a Rust backend and a Svelte 4 +
Tailwind CSS frontend, rendered through the system's WebKitGTK.

- The window runs as the live user and never as root.
- The disk work is done by the same binary in helper mode
  (`soplos-linux-installer --helper`), started with `sudo -n`. It receives the
  installation plan as JSON and reports its progress as JSON lines.

## Build

```bash
npm install
npm run tauri icon ../soplos-artwork/soplos-linux-installer/soplos-linux-installer.png
npm run tauri dev       # development
./package.sh            # builds the .deb and copies it to Descargas and MEGA
```

## Project layout

```
src/                     Svelte interface (App, steps, i18n)
src/presentation/        the presentation shown while installing, with its texts
src-tauri/src/commands/  Rust: commands for the window
src-tauri/src/helper/    Rust: the installation (plan, disk, system)
src-tauri/resources/     desktop file, copyright and the data of each edition
DESIGN.md                the design of the installer
```

## License

GPL-3.0-or-later. Copyright 2026 Sergi Perich.

## Support

- Website: https://soplos.org
- Issues: https://github.com/SoplosLinux/soplos-linux-installer-3.0/issues

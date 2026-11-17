// The partition layouts of DESIGN.md, section 5. The disk step builds here
// the exact table the disk will end with, and that same table is what the
// preview draws and what the helper carries out: what the user sees is what
// is done.
//
// A Layout lists the partitions the disk ends with, in order. Any existing
// partition that is not in the list is deleted; newTable wipes the disk
// first and writes an empty GPT.

export const MiB = 1024 ** 2;
export const GiB = 1024 ** 3;

export const GUID = {
  esp: "c12a7328-f81f-11d2-ba4b-00a0c93ec93b",
  biosGrub: "21686148-6449-6e6f-744e-656564454649",
  xbootldr: "bc13c2ff-59e6-4262-a352-b275fd6f7172",
  linux: "0fc63daf-8483-4772-8e79-3d69d8477de4",
  swap: "0657fd6d-a4ab-43c4-84e5-0933c84b4f4f",
  basicData: "ebd0a0a2-b9e5-4433-87c0-68b6b72699c7",
};

export interface Partition {
  path: string;
  start: number;
  size: number;
  fstype: string;
  fsver: string;
  label: string;
  partlabel: string;
  parttype: string;
  mountpoints: string[];
  fsused: number | null;
}

export interface Disk {
  path: string;
  model: string;
  size: number;
  transport: string;
  removable: boolean;
  table: string;
  sectorSize: number;
  partitions: Partition[];
}

export type Action = "keep" | "shrink" | "use" | "format" | "create";

export interface Planned {
  /** The existing partition, "" for a new one. */
  path: string;
  start: number;
  size: number;
  /** keep: leave it alone. shrink: make it smaller, data kept. use: mount it
   * as it is. format: new file system on an existing partition. create: new. */
  action: Action;
  /** The file system it ends with: "vfat", "ext4", "btrfs", "xfs", "f2fs",
   * "jfs", "exfat", "ntfs", "swap",
   * "" for the BIOS boot partition, or the current one when kept. */
  fs: string;
  mount: string;
  parttype: string;
  /** LUKS2 under the file system. */
  encrypt: boolean;
  /** Only to tell existing partitions apart in the preview. */
  label: string;
}

export interface Layout {
  newTable: boolean;
  /** The kind of the new table: "gpt" or "msdos" (MBR). Only with newTable. */
  table?: "gpt" | "msdos";
  parts: Planned[];
}

export type Firmware = "uefi" | "bios" | "";
export type Bootloader = "grub" | "systemd-boot" | "limine" | "";

export const alignUp = (x: number) => Math.ceil(x / MiB) * MiB;
export const alignDown = (x: number) => Math.floor(x / MiB) * MiB;

/** The first MiB holds the partition table; the last one, GPT's backup. */
export const FIRST = MiB;
export const usableEnd = (disk: Disk) => alignDown(disk.size - MiB);

/** 8 GiB. For hibernation, the RAM plus its square root (the rule of
 * Ubuntu and Red Hat), so the image fits even with the swap partly in use. */
export function swapSize(choice: "none" | "swap" | "hibernate", ramBytes: number): number {
  if (choice === "none") return 0;
  if (choice === "hibernate") {
    const ram = Math.max(Math.ceil(ramBytes / GiB), 1);
    return Math.max(ram + Math.ceil(Math.sqrt(ram)), 9) * GiB;
  }
  return 8 * GiB;
}

export const espSize = (bootloader: Bootloader) => (bootloader === "grub" ? GiB : 2 * GiB);
export const BOOT_SIZE = 2 * GiB;
/** What systemd-boot and Limine need on their FAT for two kernels. */
export const KERNELS_FAT = 2 * GiB;

export interface Options {
  firmware: Firmware;
  bootloader: Bootloader;
  filesystem: "btrfs" | "ext4" | "xfs";
  encrypt: boolean;
  swap: number;
  /** The smallest root the edition needs. */
  minRoot: number;
}

type Piece = Omit<Planned, "start" | "path" | "label"> & { path?: string; label?: string; rest?: boolean };

const fresh = (size: number, fs: string, mount: string, parttype: string, encrypt = false): Piece => ({
  action: "create",
  size,
  fs,
  mount,
  parttype,
  encrypt,
});

/** Lays the pieces out from start to end: the root takes what is left, the swap goes last. */
function place(pieces: Piece[], start: number, end: number): Planned[] | null {
  const fixed = pieces.filter((p) => !p.rest).reduce((sum, p) => sum + p.size, 0);
  const rest = alignDown(end - start - fixed);
  if (rest < 0) return null;
  const out: Planned[] = [];
  let at = start;
  for (const p of pieces) {
    const size = p.rest ? rest : p.size;
    out.push({ ...p, path: p.path ?? "", label: p.label ?? "", size, start: at, rest: undefined } as Planned);
    at += size;
  }
  return out;
}

function newSystem(o: Options, withBoot: boolean): Piece[] {
  const root: Piece = { ...fresh(0, o.filesystem, "/", GUID.linux, o.encrypt), rest: true };
  const pieces: Piece[] = [];
  if (withBoot) pieces.push(fresh(BOOT_SIZE, "ext4", "/boot", GUID.linux));
  pieces.push(root);
  if (o.swap > 0) pieces.push(fresh(o.swap, "swap", "", GUID.swap));
  return pieces;
}

/** The boot partitions a new GPT needs (DESIGN.md, 5.1). */
function bootPieces(o: Options): Piece[] {
  if (o.firmware === "uefi") return [fresh(espSize(o.bootloader), "vfat", "/boot/efi", GUID.esp)];
  const pieces = [fresh(MiB, "", "", GUID.biosGrub)];
  if (o.bootloader === "limine") pieces.push(fresh(KERNELS_FAT, "vfat", "/boot/limine", GUID.basicData));
  return pieces;
}

/** Space the new system needs, without the root. */
export function fixedNeed(o: Options): number {
  return bootPieces(o).reduce((s, p) => s + p.size, 0) + BOOT_SIZE + o.swap;
}

export function eraseLayout(disk: Disk, o: Options): Layout | null {
  const parts = place([...bootPieces(o), ...newSystem(o, true)], FIRST, usableEnd(disk));
  if (!parts) return null;
  const root = parts.find((p) => p.mount === "/");
  if (!root || root.size < o.minRoot) return null;
  return { newTable: true, table: "gpt", parts };
}

export const isEsp = (p: Partition) => p.parttype.toLowerCase() === GUID.esp || p.parttype.toLowerCase() === "0xef";

const keep = (p: Partition): Planned => ({
  path: p.path,
  start: p.start,
  size: p.size,
  action: "keep",
  fs: p.fstype,
  mount: "",
  parttype: p.parttype,
  encrypt: false,
  label: p.label || p.partlabel,
});

/** The swap partition already on the disk. Next to another system Soplos
 * shares it, as it is: one swap for every system of the machine, never a
 * second one. */
export const existingSwap = (disk: Disk) => disk.partitions.find((p) => p.fstype === "swap");

/** What has to be added next to an existing system, beyond /boot, root and swap. */
function alongsideExtras(disk: Disk, o: Options): { pieces: Piece[]; separateBoot: boolean; esp?: Partition; swap?: Partition } {
  const pieces: Piece[] = [];
  let separateBoot = true;
  const esp = disk.partitions.find(isEsp);
  const swap = existingSwap(disk);
  if (o.firmware === "uefi") {
    if (!esp) pieces.push(fresh(espSize(o.bootloader), "vfat", "/boot/efi", GUID.esp));
    else if (o.bootloader === "systemd-boot" && esp.size < KERNELS_FAT) {
      // Windows makes ESPs of 100-260 MiB: the kernels go to an XBOOTLDR,
      // which systemd-boot reads as /boot.
      pieces.push(fresh(KERNELS_FAT, "vfat", "/boot", GUID.xbootldr));
      separateBoot = false;
    } else if (o.bootloader === "limine" && esp.size < KERNELS_FAT) {
      pieces.push(fresh(KERNELS_FAT, "vfat", "/boot/limine", GUID.basicData));
    }
  } else {
    if (disk.table === "gpt" && !disk.partitions.some((p) => p.parttype.toLowerCase() === GUID.biosGrub)) {
      pieces.push(fresh(MiB, "", "", GUID.biosGrub));
    }
    if (o.bootloader === "limine") pieces.push(fresh(KERNELS_FAT, "vfat", "/boot/limine", GUID.basicData));
  }
  return { pieces, separateBoot, esp, swap };
}

/** Space Soplos needs next to another system, root included. */
export function alongsideNeed(disk: Disk, o: Options): number {
  const { pieces, separateBoot, swap } = alongsideExtras(disk, o);
  return pieces.reduce((s, p) => s + p.size, 0) + (separateBoot ? BOOT_SIZE : 0) + (swap ? 0 : o.swap) + o.minRoot;
}

/** The free space that follows a partition, up to the next one or the end. */
export function spaceAfter(disk: Disk, part: Partition): number {
  const next = disk.partitions.find((p) => p.start > part.start);
  return (next ? next.start : usableEnd(disk)) - (part.start + part.size);
}

/** Shrinks `target` to `newSize` and installs in the space freed after it. */
export function alongsideLayout(disk: Disk, target: Partition, newSize: number, o: Options): Layout | null {
  const { pieces, separateBoot, esp, swap } = alongsideExtras(disk, o);
  const start = alignUp(target.start + newSize);
  const next = disk.partitions.find((p) => p.start > target.start);
  const end = next ? alignDown(next.start) : usableEnd(disk);
  const placed = place([...pieces, ...newSystem(swap ? { ...o, swap: 0 } : o, separateBoot)], start, end);
  if (!placed) return null;
  const root = placed.find((p) => p.mount === "/");
  if (!root || root.size < o.minRoot) return null;
  const existing = disk.partitions.map((p) => {
    if (p.path === target.path) return { ...keep(p), action: "shrink" as Action, size: newSize };
    if (o.firmware === "uefi" && esp && p.path === esp.path) return { ...keep(p), action: "use" as Action, mount: "/boot/efi" };
    // Shared as it is: the helper does not format (nor encrypt) a swap it is told to use.
    if (swap && p.path === swap.path) return { ...keep(p), action: "use" as Action, fs: "swap" };
    return keep(p);
  });
  return { newTable: false, parts: [...existing, ...placed].sort((a, b) => a.start - b.start) };
}

/** The partitions an existing system can give space from. */
export const SHRINKABLE = ["ntfs", "ext4", "ext3", "ext2", "btrfs"];

// --- Manual -------------------------------------------------------------

export interface Gap {
  start: number;
  size: number;
}

/** The free stretches of at least 1 MiB in a layout. */
export function gaps(disk: Disk, layout: Layout): Gap[] {
  const out: Gap[] = [];
  let at = FIRST;
  for (const p of [...layout.parts].sort((a, b) => a.start - b.start)) {
    const start = alignUp(at);
    if (p.start - start >= MiB) out.push({ start, size: alignDown(p.start - start) });
    at = Math.max(at, p.start + p.size);
  }
  const start = alignUp(at);
  if (usableEnd(disk) - start >= MiB) out.push({ start, size: usableEnd(disk) - start });
  return out;
}

/** The disk as it is: every partition kept. */
export function currentLayout(disk: Disk): Layout {
  // A disk without a table gets a GPT one.
  if (!disk.table) return { newTable: true, table: "gpt", parts: [] };
  return { newTable: false, parts: disk.partitions.map(keep) };
}

// --- Mount points and file systems ----------------------------------------

/** The mount points of the system that can have a partition of their own,
 * as Calamares offers them. */
export const SYSTEM_MOUNTS = ["/", "/boot", "/home", "/opt", "/srv", "/usr", "/var"];

/** What the installed system can live on. vfat, exFAT and NTFS keep no Unix
 * permissions, so they are only for data. */
export const LINUX_FS = ["btrfs", "ext4", "xfs", "f2fs", "jfs"];
/** For a partition of data (a mount point of the user's own). */
export const DATA_FS = [...LINUX_FS, "exfat", "ntfs", "vfat"];

/** Where the system itself lives, or a tool of the system: never a mount point of data. */
const RESERVED = ["/bin", "/boot", "/dev", "/etc", "/lib", "/lib32", "/lib64", "/libx32", "/proc", "/root", "/run", "/sbin", "/sys", "/target"];

/** A mount point of the user's own: an absolute path with plain characters,
 * not one of the system's. */
export function validMount(path: string): boolean {
  if (!/^\/[A-Za-z0-9._-]+(\/[A-Za-z0-9._-]+)*$/.test(path) || path.length > 200) return false;
  if (path.split("/").some((piece) => piece === "." || piece === "..")) return false;
  if (SYSTEM_MOUNTS.includes(path)) return true;
  return !RESERVED.some((r) => path === r || path.startsWith(r + "/"));
}

/** A mount point that is not one of the system's: a partition of data. */
export const isDataMount = (mount: string) => mount !== "" && !SYSTEM_MOUNTS.includes(mount) && mount !== "/boot/efi" && mount !== "/boot/limine";

export function parttypeFor(fs: string, mount: string): string {
  if (mount === "/boot/efi") return GUID.esp;
  if (fs === "swap") return GUID.swap;
  if (fs === "vfat" || fs === "exfat" || fs === "ntfs") return GUID.basicData;
  if (fs === "biosgrub") return GUID.biosGrub;
  return GUID.linux;
}

/** The problems of a manual layout (DESIGN.md, 5.2), as i18n keys. */
export function manualProblems(layout: Layout, o: Pick<Options, "firmware" | "bootloader" | "minRoot">, table: string): string[] {
  const problems: string[] = [];
  const at = (mount: string) => layout.parts.find((p) => p.mount === mount);
  const root = at("/");
  const boot = at("/boot");
  const esp = at("/boot/efi");
  if (!root) problems.push("noRoot");
  else if (root.size < o.minRoot) problems.push("rootSmall");
  const mounts = layout.parts.map((p) => p.mount).filter(Boolean);
  if (new Set(mounts).size !== mounts.length) problems.push("duplicateMount");
  if (o.firmware === "uefi") {
    if (!esp || esp.fs !== "vfat") problems.push("noEsp");
    // systemd-boot and Limine keep the kernels on the ESP, unless /boot is
    // a FAT of its own (XBOOTLDR).
    else if (o.bootloader !== "grub" && !(boot && boot.fs === "vfat") && esp.size < (boot ? boot.size : KERNELS_FAT) - MiB)
      problems.push("espSmall");
  } else {
    const gpt = layout.newTable ? layout.table !== "msdos" : table === "gpt";
    if (gpt && !layout.parts.some((p) => p.parttype === GUID.biosGrub)) problems.push("noBiosGrub");
    if (o.bootloader === "limine" && at("/boot/limine")?.fs !== "vfat") problems.push("noLimineFat");
  }
  // A partition kept as it is must already have a file system its use can live on.
  for (const p of layout.parts) {
    if (SYSTEM_MOUNTS.includes(p.mount) && p.action === "use" && !LINUX_FS.includes(p.fs) && !(p.mount === "/boot" && p.fs === "vfat")) {
      problems.push("needsFormat");
      break;
    }
  }
  if (layout.parts.some((p) => isDataMount(p.mount) && !validMount(p.mount))) problems.push("badMount");
  if (layout.parts.some((p) => isDataMount(p.mount) && p.action === "use" && !DATA_FS.includes(p.fs))) problems.push("unknownFs");
  // MBR: four primary partitions at most (no extended ones are made) and
  // no disks beyond 2 TiB.
  const mbr = layout.newTable ? layout.table === "msdos" : table === "dos";
  if (mbr && layout.parts.length > 4) problems.push("mbrFour");
  return problems;
}

/** MBR cannot address more than 2 TiB. */
export const MBR_LIMIT = 2 * 1024 * GiB;

// --- Preview --------------------------------------------------------------

export type Role = "esp" | "biosgrub" | "boot" | "kernels" | "root" | "home" | "swap" | "other" | "existing";

export function roleOf(p: Planned): Role {
  if (p.parttype === GUID.biosGrub) return "biosgrub";
  if (p.mount === "/boot/efi") return "esp";
  if (p.mount === "/boot/limine" || p.parttype === GUID.xbootldr) return "kernels";
  if (p.mount === "/boot") return "boot";
  if (p.mount === "/") return "root";
  if (p.mount === "/home") return "home";
  if (p.fs === "swap" && p.action !== "keep") return "swap";
  if (p.action === "keep" || p.action === "shrink") return "existing";
  return "other";
}

/** The color of each file system, the same before and after. */
export const FS_COLOR: Record<string, string> = {
  vfat: "#fec431", // Soplos secondary
  btrfs: "#ff7d2e", // Soplos primary
  ext4: "#ffa766",
  ext3: "#ffa766",
  ext2: "#ffa766",
  xfs: "#ef5a33", // Soplos accent
  f2fs: "#e8954f",
  jfs: "#c96f3b",
  exfat: "#d4b04a",
  ntfs: "#8e8e93",
  swap: "#5c5c61",
  "": "#b8b8bd",
};
/** The name shown for a file system: vfat is what Linux calls FAT32. */
const FS_NAMES: Record<string, string> = { vfat: "FAT32", ntfs: "NTFS", exfat: "exFAT", f2fs: "F2FS", jfs: "JFS" };
export const fsLabel = (fs: string) => FS_NAMES[fs] ?? (fs || "—");

export const fsColor = (fs: string) => FS_COLOR[fs] ?? "#a3a3a8";

/** Dark text on light blocks, white on dark ones. */
export function inkFor(hex: string): string {
  const n = parseInt(hex.slice(1), 16);
  const [r, g, b] = [(n >> 16) & 255, (n >> 8) & 255, n & 255];
  return 0.299 * r + 0.587 * g + 0.114 * b > 150 ? "rgba(0,0,0,0.8)" : "rgba(255,255,255,0.92)";
}

export interface BarItem {
  start: number;
  size: number;
  color: string;
  /** "/", "EFI", "NTFS"... */
  name: string;
  /** "250 GiB" */
  detail: string;
  /** A line for the legend: device, label, what happens to it. */
  note?: string;
  /** Drawn faded: it will be deleted, or it is not touched. */
  faded?: boolean;
}

export function formatSize(bytes: number, locale: string): string {
  const fmt = (n: number) => new Intl.NumberFormat(locale, { maximumFractionDigits: n >= 100 ? 0 : 1 }).format(n);
  if (bytes >= GiB * 1024) return `${fmt(bytes / (GiB * 1024))} TiB`;
  if (bytes >= GiB) return `${fmt(bytes / GiB)} GiB`;
  return `${fmt(bytes / MiB)} MiB`;
}

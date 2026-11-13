import { writable } from "svelte/store";
import type { Layout } from "./partitioning";

/**
 * The steps of the installer, in order. The first six collect the plan,
 * "install" runs it and "finished" closes the installation. The sidebar lists
 * them all; only the steps already reached can be revisited, and none once
 * the installation has started.
 *
 * The welcome step stays even though Soplos Welcome Live greets the user
 * first: the installer must also work on its own, as Calamares does, so
 * another distribution could use it.
 */
export const STEPS = ["welcome", "location", "bootloader", "disk", "user", "summary", "install", "finished"] as const;
export type StepId = (typeof STEPS)[number];

/** The index of the step on screen. */
export const current = writable<number>(0);

/** The furthest step reached, so the sidebar knows which ones can be opened. */
export const reached = writable<number>(0);

/**
 * Whether the step on screen lets the user go on. Each view sets it from its
 * own checks (a blocked option, a missing field); true until a view says no.
 */
export const canContinue = writable<boolean>(true);

/**
 * What the user chooses along the way. It is sent to the helper as one JSON
 * document when the installation starts (DESIGN.md, section 2). The fields
 * are filled in as the steps get their real content.
 */
export interface InstallPlan {
  /** LANG of the installed system, "es_ES.UTF-8". */
  locale: string;
  /** LC_TIME, LC_NUMERIC, LC_MONETARY... of the installed system. */
  formats: string;
  timezone: string;
  keyboard: { layout: string; variant: string };
  firmware: "uefi" | "bios" | "";
  bootloader: "grub" | "systemd-boot" | "limine" | "";
  disk: {
    device: string;
    mode: "erase" | "alongside" | "manual" | "";
    filesystem: "btrfs" | "ext4" | "xfs";
    encrypt: boolean;
    /** The LUKS passphrase; only in memory, sent to the helper on stdin. */
    passphrase: string;
    swap: "none" | "swap" | "hibernate";
    /** Alongside: the partition that gives space and the size it keeps. */
    shrink: { partition: string; newSize: number } | null;
    /** The table the disk ends with (lib/partitioning.ts). */
    layout: Layout | null;
  };
  user: {
    fullName: string;
    login: string;
    hostname: string;
    /** Only in memory; sent to the helper on stdin. */
    password: string;
    sameRootPassword: boolean;
    /** Used only when sameRootPassword is false. */
    rootPassword: string;
  };
}

export const plan = writable<InstallPlan>({
  locale: "",
  formats: "",
  timezone: "",
  keyboard: { layout: "", variant: "" },
  firmware: "",
  bootloader: "grub",
  disk: { device: "", mode: "", filesystem: "btrfs", encrypt: false, passphrase: "", swap: "swap", shrink: null, layout: null },
  user: { fullName: "", login: "", hostname: "", password: "", sameRootPassword: true, rootPassword: "" },
});

/** The manual partition table while the user edits it (views/Disk.svelte). */
export const manualDraft = writable<Layout | null>(null);

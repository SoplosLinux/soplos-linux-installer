<script lang="ts">
  import { _, locale } from "svelte-i18n";
  import { Plus, Trash2, RotateCcw, FilePlus2 } from "lucide-svelte";
  import Select from "./Select.svelte";
  import {
    type Disk,
    type Layout,
    type Planned,
    type Firmware,
    type Bootloader,
    GiB,
    MiB,
    GUID,
    alignDown,
    MBR_LIMIT,
    gaps,
    currentLayout,
    parttypeFor,
    LINUX_FS,
    DATA_FS,
    fsColor,
    fsLabel,
    formatSize,
  } from "../lib/partitioning";

  // The manual table (DESIGN.md, 5.2): each partition gets a use and, if it
  // is written, a file system; free space can take new partitions. Nothing
  // is touched until the installation starts.
  export let disk: Disk;
  export let layout: Layout;
  export let firmware: Firmware;
  export let bootloader: Bootloader;
  export let onChange: (layout: Layout) => void;

  $: size = (bytes: number) => formatSize(bytes, $locale ?? "en");

  // "custom": a mount point the user writes, for a partition of data.
  type Use = "" | "/" | "/home" | "/boot" | "/opt" | "/srv" | "/usr" | "/var" | "/boot/efi" | "/boot/limine" | "swap" | "biosgrub" | "custom";
  /** What a mount point of the user's own starts with. */
  const DATA_MOUNT = "/mnt/data";
  // Only the uses the chosen boot loader has: /boot/efi on UEFI, the BIOS
  // boot partition on BIOS, and the FAT of Limine only for Limine on BIOS
  // (on UEFI its kernels go to the ESP).
  $: USES = [
    "",
    "/",
    "/home",
    "/boot",
    // The rest of the mount points Calamares offers, for those who give each one its own partition.
    "/opt",
    "/srv",
    "/usr",
    "/var",
    ...(firmware === "bios" ? [] : ["/boot/efi"]),
    ...(firmware === "bios" && bootloader === "limine" ? ["/boot/limine"] : []),
    "swap",
    // The BIOS boot partition exists only on GPT.
    ...(firmware === "bios" && kind === "gpt" ? ["biosgrub"] : []),
    "custom",
  ] as Use[];

  /** The table the disk ends with: the new one, or the one it has. */
  $: kind = layout.newTable ? (layout.table ?? "gpt") : disk.table === "dos" ? "msdos" : "gpt";
  $: mbrAllowed = disk.size <= MBR_LIMIT;

  $: unchanged = JSON.stringify(layout) === JSON.stringify(currentLayout(disk));

  function newTable(table: "gpt" | "msdos") {
    creatingAt = null;
    onChange({ newTable: true, table, parts: [] });
  }
  const USE_KEY: Record<Use, string> = {
    "": "none",
    "/": "root",
    "/home": "home",
    "/boot": "boot",
    "/opt": "opt",
    "/srv": "srv",
    "/usr": "usr",
    "/var": "var",
    "/boot/efi": "esp",
    "/boot/limine": "limine",
    swap: "swap",
    biosgrub: "biosgrub",
    custom: "custom",
  };
  $: useOptions = USES.map((u) => ({ value: u, label: $_(`disk.uses.${USE_KEY[u]}`) }));

  function fsChoices(use: Use): string[] {
    if (use === "/boot/efi" || use === "/boot/limine") return ["vfat"];
    if (use === "swap") return ["swap"];
    if (use === "biosgrub") return [""];
    // The first one is the default: /boot is ext4, as in the automatic layouts (DESIGN.md).
    if (use === "/boot") return ["ext4", "btrfs", "xfs", "vfat"];
    // Data: also the file systems Windows and macOS read.
    if (use === "custom" || use === "") return DATA_FS;
    return LINUX_FS;
  }

  const KNOWN = new Set<string>(["", "/", "/home", "/boot", "/opt", "/srv", "/usr", "/var", "/boot/efi", "/boot/limine"]);
  const useOf = (p: Planned): Use =>
    p.parttype === GUID.biosGrub
      ? "biosgrub"
      : p.fs === "swap" && p.action !== "keep"
        ? "swap"
        : KNOWN.has(p.mount)
          ? (p.mount as Use)
          : "custom";
  const writes = (p: Planned) => p.action === "create" || p.action === "format";
  const original = (p: Planned) => disk.partitions.find((d) => d.path === p.path);

  /** A partition with a new use, written or not with the file system given.
   * `custom` is the mount point of a "custom" use. */
  function rebuild(p: Planned, use: Use, format: boolean, fs: string, custom = DATA_MOUNT): Planned {
    const isNew = !p.path;
    const choices = fsChoices(use);
    // Some uses need their own file system: a partition that does not have
    // it has to be written.
    const current = original(p)?.fstype ?? "";
    const mustFormat = isNew || (use !== "" && use !== "biosgrub" && !choices.includes(current));
    const write = use === "biosgrub" ? isNew : format || mustFormat;
    const finalFs = use === "biosgrub" ? "" : write ? (choices.includes(fs) ? fs : choices[0]) : current;
    const mount = use === "swap" || use === "biosgrub" ? "" : use === "custom" ? custom : use;
    return {
      ...p,
      mount,
      fs: finalFs,
      action: isNew ? "create" : write ? "format" : use ? "use" : "keep",
      parttype: use === "biosgrub" ? GUID.biosGrub : write ? parttypeFor(finalFs, mount) : mount === "/boot/efi" ? GUID.esp : (original(p)?.parttype ?? p.parttype),
    };
  }

  // A new use starts with its own default file system (ext4 for /boot), not
  // with the one the partition had for the previous use.
  function setUse(index: number, p: Planned, value: string) {
    const use = value as Use;
    const same = use === useOf(p);
    update(index, rebuild(p, use, writes(p), same ? p.fs : fsChoices(use)[0], same ? p.mount : DATA_MOUNT));
  }

  /** The mount point of a partition of data, as the user types it. */
  function setMount(index: number, p: Planned, value: string) {
    update(index, { ...p, mount: value.trim() });
  }

  function chooseNewUse(value: string) {
    newUse = value as Use;
    newFs = fsChoices(newUse)[0];
    newMount = DATA_MOUNT;
    newSizeGiB = toGiB(usualSize(newUse, creatingSize));
    // The swap goes to the end of the disk, the rest from the start.
    newAt = newUse === "swap" ? "end" : "start";
  }

  function update(index: number, next: Planned) {
    const parts = layout.parts.slice();
    parts[index] = next;
    onChange({ ...layout, parts });
  }

  function remove(index: number) {
    onChange({ ...layout, parts: layout.parts.filter((_, i) => i !== index) });
  }

  // --- New partitions ----------------------------------------------------

  let creatingAt: number | null = null;
  let creatingSize = 0;
  let newSizeGiB = 0;
  let newUse: Use = "/";
  let newFs = "btrfs";
  let newMount = DATA_MOUNT;
  /** Where the new partition goes in the free space. */
  let newAt: "start" | "end" = "start";

  /** The usual size of each use: the rest of the space for / and /home. */
  function usualSize(use: Use, gap: number): number {
    const usual: Partial<Record<Use, number>> = {
      "/boot/efi": bootloader === "grub" ? GiB : 2 * GiB,
      "/boot": 2 * GiB,
      "/boot/limine": 2 * GiB,
      swap: 8 * GiB,
      biosgrub: MiB,
    };
    return Math.min(usual[use] ?? gap, gap);
  }

  const toGiB = (bytes: number) => Math.floor((bytes / GiB) * 100) / 100;

  function startCreate(gapStart: number, gapSize: number) {
    creatingAt = gapStart;
    creatingSize = gapSize;
    chooseNewUse(layout.parts.some((p) => p.mount === "/") ? "/home" : "/");
  }

  function create(gapSize: number) {
    if (creatingAt === null) return;
    let bytes = Math.min(alignDown(newSizeGiB * GiB), gapSize);
    if (bytes < MiB) return;
    // The size is typed in GiB with two decimals, which falls short of the
    // real free space by a few MiB. A leftover that small is useless: the
    // partition takes it, so no crumbs of free space are left.
    if (gapSize - bytes < 16 * MiB) bytes = gapSize;
    // At the end: it ends where the free space ends, as GParted does when the
    // block is slid to the right.
    const start = newAt === "end" ? alignDown(creatingAt + gapSize - bytes) : creatingAt;
    const blank: Planned = { path: "", start, size: bytes, action: "create", fs: newFs, mount: "", parttype: GUID.linux, encrypt: false, label: "" };
    const part = rebuild(blank, newUse, true, newFs, newMount.trim());
    onChange({ ...layout, parts: [...layout.parts, part].sort((a, b) => a.start - b.start) });
    creatingAt = null;
  }

  // Partitions and free space, in the order they lie on the disk.
  $: rows = [
    ...layout.parts.map((p, index) => ({ kind: "part" as const, start: p.start, p, index })),
    ...gaps(disk, layout).map((g) => ({ kind: "gap" as const, start: g.start, gap: g })),
  ].sort((a, b) => a.start - b.start);

  function name(p: Planned) {
    if (!p.path) return $_("disk.newPartition");
    const o = original(p);
    return o?.label || o?.partlabel ? `${p.path} (${o.label || o.partlabel})` : p.path;
  }
</script>

<div class="h-full min-h-0 flex flex-col rounded-lg border border-border bg-surface">
  <div class="shrink-0 flex items-center gap-2 px-4 py-2 border-b border-border">
    <span class="text-[0.8rem] font-semibold uppercase tracking-wide text-text-muted">{$_("disk.table")}</span>
    <!-- The kind of table. Picking one writes a new, empty table. -->
    <span class="flex rounded border border-border overflow-hidden text-[0.78rem]">
      {#each [["gpt", "GPT"], ["msdos", "MBR"]] as [value, label]}
        <button
          type="button"
          disabled={value === "msdos" && !mbrAllowed}
          title={value === "msdos" && !mbrAllowed ? $_("disk.mbrTooBig") : ""}
          class="px-2.5 py-1 {kind === value ? 'bg-primary text-white font-semibold' : 'text-text hover:bg-surface-2 disabled:opacity-40 disabled:hover:bg-transparent'}"
          on:click={() => kind !== value && newTable(value === "msdos" ? "msdos" : "gpt")}
        >
          {label}
        </button>
      {/each}
    </span>
    <span class="flex-1 min-w-0 truncate text-[0.75rem] {layout.newTable && disk.partitions.length ? 'text-danger' : 'text-text-muted'}">
      {layout.newTable && disk.partitions.length ? $_("disk.newTableActive") : ""}
    </span>
    <!-- Emptying the table only means something when it has partitions. -->
    {#if layout.parts.length}
      <button
        type="button"
        class="flex items-center gap-1.5 rounded px-2.5 py-1 text-[0.8rem] text-text hover:bg-danger/15 hover:text-danger"
        on:click={() => newTable(kind === "msdos" ? "msdos" : "gpt")}
      >
        <FilePlus2 size={14} />
        {$_("disk.deleteAll")}
      </button>
    {/if}
    <button
      type="button"
      disabled={unchanged}
      class="flex items-center gap-1.5 rounded px-2.5 py-1 text-[0.8rem] text-text hover:bg-surface-2 hover:text-text-bright disabled:opacity-40 disabled:hover:bg-transparent disabled:hover:text-text"
      on:click={() => {
        creatingAt = null;
        onChange(currentLayout(disk));
      }}
    >
      <RotateCcw size={14} />
      {$_("disk.reset")}
    </button>
  </div>

  <ul class="flex-1 min-h-0 overflow-y-auto divide-y divide-border list-scroll">
    {#each rows as row (row.kind + row.start)}
      {#if row.kind === "part"}
        {@const p = row.p}
        {@const use = useOf(p)}
        <li class="grid grid-cols-[minmax(0,1.3fr)_minmax(0,1fr)_minmax(0,1fr)_auto] items-center gap-3 px-4 py-2">
          <div class="flex items-center gap-2 min-w-0">
            <span class="w-2.5 h-2.5 rounded-sm shrink-0" style="background: {fsColor(p.fs)}"></span>
            <div class="min-w-0">
              <p class="text-[0.85rem] text-text-bright truncate">{name(p)}</p>
              <p class="text-[0.75rem] text-text-muted">
                {size(p.size)}{original(p)?.fstype ? ` · ${fsLabel(original(p)?.fstype ?? "")}` : ""}
              </p>
            </div>
          </div>
          <div class="flex flex-col gap-1 min-w-0">
            <Select label={$_("disk.use")} value={use} options={useOptions} onChange={(v) => setUse(row.index, p, v)} />
            {#if use === "custom"}
              <input
                type="text"
                spellcheck="false"
                aria-label={$_("disk.mountPoint")}
                placeholder={DATA_MOUNT}
                value={p.mount}
                on:change={(e) => setMount(row.index, p, e.currentTarget.value)}
                class="w-full bg-surface-2 border border-border rounded px-2 py-1 text-[0.82rem] text-text-bright outline-none focus-visible:ring-2 focus-visible:ring-primary/60"
              />
            {/if}
          </div>
          {#if use === "biosgrub"}
            <span></span>
          {:else if p.path}
            <label class="flex items-center gap-2 min-w-0">
              <input
                type="checkbox"
                class="w-4 h-4 accent-primary shrink-0"
                checked={writes(p)}
                on:change={(e) => update(row.index, rebuild(p, use, e.currentTarget.checked, p.fs || fsChoices(use)[0], p.mount))}
              />
              {#if writes(p)}
                <div class="flex-1 min-w-0">
                  <Select
                    label={$_("disk.filesystem")}
                    value={p.fs}
                    options={fsChoices(use).map((f) => ({ value: f, label: fsLabel(f) }))}
                    onChange={(v) => update(row.index, rebuild(p, use, true, v, p.mount))}
                  />
                </div>
              {:else}
                <span class="text-[0.8rem] text-text truncate">{$_("disk.format")}</span>
              {/if}
            </label>
          {:else}
            <Select
              label={$_("disk.filesystem")}
              value={p.fs}
              options={fsChoices(use).map((f) => ({ value: f, label: fsLabel(f) }))}
              onChange={(v) => update(row.index, rebuild(p, use, true, v, p.mount))}
            />
          {/if}
          <button
            type="button"
            class="grid place-items-center w-8 h-8 rounded text-text-muted hover:bg-danger/15 hover:text-danger"
            title={$_("disk.delete")}
            aria-label={$_("disk.delete")}
            on:click={() => remove(row.index)}
          >
            <Trash2 size={15} />
          </button>
        </li>
      {:else}
        {@const g = row.gap}
        <li class="px-4 py-2">
          {#if creatingAt === g.start}
            <div class="grid grid-cols-[minmax(0,1.3fr)_minmax(0,1fr)_minmax(0,1fr)_auto] items-center gap-3">
              <label class="flex items-center gap-2 text-[0.82rem] text-text">
                <input
                  type="number"
                  min="0.1"
                  step="0.01"
                  max={toGiB(g.size)}
                  bind:value={newSizeGiB}
                  class="w-24 bg-surface-2 border border-border rounded px-2 py-1.5 text-[0.85rem] text-text-bright outline-none focus-visible:ring-2 focus-visible:ring-primary/60"
                />
                GiB
              </label>
              <div class="flex flex-col gap-1 min-w-0">
                <Select
                  label={$_("disk.use")}
                  value={newUse}
                  options={useOptions}
                  onChange={chooseNewUse}
                />
                {#if newUse === "custom"}
                  <input
                    type="text"
                    spellcheck="false"
                    aria-label={$_("disk.mountPoint")}
                    placeholder={DATA_MOUNT}
                    bind:value={newMount}
                    class="w-full bg-surface-2 border border-border rounded px-2 py-1 text-[0.82rem] text-text-bright outline-none focus-visible:ring-2 focus-visible:ring-primary/60"
                  />
                {/if}
              </div>
              <Select
                label={$_("disk.filesystem")}
                value={newFs}
                options={fsChoices(newUse).map((f) => ({ value: f, label: fsLabel(f) }))}
                onChange={(v) => (newFs = v)}
              />
              <div class="flex gap-1">
                <span class="flex rounded border border-border overflow-hidden text-[0.78rem] mr-1">
                  {#each ["start", "end"] as at}
                    <button
                      type="button"
                      class="px-2 py-1 whitespace-nowrap {newAt === at ? 'bg-primary text-white font-semibold' : 'text-text hover:bg-surface-2'}"
                      on:click={() => (newAt = at === "end" ? "end" : "start")}
                    >
                      {$_(at === "end" ? "disk.atEnd" : "disk.atStart")}
                    </button>
                  {/each}
                </span>
                <button type="button" class="rounded px-3 py-1.5 text-[0.8rem] bg-primary text-white hover:bg-primary-hover" on:click={() => create(g.size)}>
                  {$_("disk.add")}
                </button>
                <button type="button" class="rounded px-2.5 py-1.5 text-[0.8rem] text-text hover:bg-surface-2" on:click={() => (creatingAt = null)}>
                  {$_("common.cancel")}
                </button>
              </div>
            </div>
          {:else}
            <div class="flex items-center gap-2">
              <span class="w-2.5 h-2.5 rounded-sm shrink-0 border border-border"></span>
              <span class="text-[0.85rem] text-text">{$_("disk.freeSpace")}</span>
              <span class="text-[0.75rem] text-text-muted">{size(g.size)}</span>
              <button
                type="button"
                class="ml-auto flex items-center gap-1.5 rounded px-2.5 py-1 text-[0.8rem] text-primary hover:bg-primary/10"
                on:click={() => startCreate(g.start, g.size)}
              >
                <Plus size={14} />
                {$_("disk.create")}
              </button>
            </div>
          {/if}
        </li>
      {/if}
    {/each}
  </ul>
</div>

<style>
  .list-scroll {
    scrollbar-gutter: stable;
  }
</style>

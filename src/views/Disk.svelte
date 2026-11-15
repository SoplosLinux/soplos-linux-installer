<script lang="ts">
  import { onMount } from "svelte";
  import { _, locale } from "svelte-i18n";
  import { HardDrive, Eraser, Columns2, SlidersHorizontal, Lock, LoaderCircle, TriangleAlert, Check, CircleCheck, Circle } from "lucide-svelte";
  import type { ComponentType } from "svelte";
  import { api } from "../lib/api";
  import { get } from "svelte/store";
  import { plan, canContinue, manualDraft, type InstallPlan } from "../lib/wizard";
  import {
    type Disk,
    type Partition,
    type Layout,
    type Planned,
    type Options,
    GiB,
    MiB,
    alignUp,
    alignDown,
    swapSize,
    existingSwap,
    eraseLayout,
    alongsideLayout,
    alongsideNeed,
    spaceAfter,
    fixedNeed,
    currentLayout,
    manualProblems,
    fsColor,
    fsLabel,
    isEsp,
    GUID,
    SHRINKABLE,
    formatSize,
  } from "../lib/partitioning";
  import Select from "../components/Select.svelte";
  import PartitionBar from "../components/PartitionBar.svelte";
  import ManualPartitioner from "../components/ManualPartitioner.svelte";

  type Mode = InstallPlan["disk"]["mode"];

  let disks: Disk[] = [];
  let loaded = false;
  let ramBytes = 0;
  let minRoot = 0;

  onMount(async () => {
    const [list, req] = await Promise.all([
      api.invoke<Disk[]>("list_disks").catch(() => [] as Disk[]),
      api.invoke<{ memoryGb: number | null; diskRequiredGb: number | null }>("check_requirements").catch(() => null),
    ]);
    disks = list;
    ramBytes = (req?.memoryGb ?? 0) * GiB;
    // The disk requirement of the edition covers the whole system: the root
    // gets it minus the boot partitions (3 GiB at most). The swap is extra.
    minRoot = Math.max((req?.diskRequiredGb ?? 0) * 1e9 - 3 * GiB, 0);
    loaded = true;
    if (!disks.some((d) => d.path === $plan.disk.device)) {
      const first = disks.find((d) => !d.removable) ?? disks[0];
      if (first) selectDisk(first.path);
    }
  });

  $: ui = $locale ?? "en";
  $: size = (bytes: number) => formatSize(bytes, ui);
  $: disk = disks.find((d) => d.path === $plan.disk.device);

  function setDisk(patch: Partial<InstallPlan["disk"]>) {
    plan.update((p) => ({ ...p, disk: { ...p.disk, ...patch } }));
  }

  const setFilesystem = (v: string) => setDisk({ filesystem: v as InstallPlan["disk"]["filesystem"] });
  const setSwap = (v: string) => setDisk({ swap: v as InstallPlan["disk"]["swap"] });

  function selectDisk(path: string) {
    // A new disk starts with no mode: nothing is erased without choosing it.
    setDisk({ device: path, mode: "", shrink: null, layout: null });
  }

  $: diskOptions = disks.map((d) => ({
    value: d.path,
    label: `${d.model || d.path} · ${size(d.size)} · ${d.path}`,
  }));

  $: options = {
    firmware: $plan.firmware,
    bootloader: $plan.bootloader,
    filesystem: $plan.disk.filesystem,
    encrypt: $plan.disk.encrypt,
    swap: swapSize($plan.disk.swap, ramBytes),
    minRoot,
  } satisfies Options;

  // --- Modes ---------------------------------------------------------------

  const MODES: { id: Exclude<Mode, "">; icon: ComponentType }[] = [
    { id: "erase", icon: Eraser },
    { id: "alongside", icon: Columns2 },
    { id: "manual", icon: SlidersHorizontal },
  ];

  /** Partitions that can give space: a known file system, not in use, with room for Soplos. */
  $: candidates = disk
    ? disk.partitions.filter(
        (p) =>
          SHRINKABLE.includes(p.fstype) &&
          p.mountpoints.length === 0 &&
          p.size + spaceAfter(disk!, p) > alongsideNeed(disk!, options) + GiB,
      )
    : [];
  // Whether the system fits at all; the swap can still be left out.
  $: eraseFits = disk ? disk.size >= fixedNeed({ ...options, swap: 0 }) + minRoot + 2 * MiB : false;

  function available(mode: Mode) {
    if (!disk) return false;
    if (mode === "erase") return eraseFits;
    if (mode === "alongside") return candidates.length > 0;
    return true;
  }

  function chooseMode(mode: Mode) {
    if (!available(mode) || !disk) return;
    if (mode === "manual") {
      raw = currentLayout(disk);
      setDisk({ mode, shrink: null, layout: null });
    }
    else if (mode === "alongside") {
      const target = candidates.find((p) => p.path === $plan.disk.shrink?.partition) ?? biggest(candidates);
      setDisk({ mode, shrink: null, layout: null });
      chooseTarget(target.path);
    } else setDisk({ mode, shrink: null, layout: null });
  }

  const biggest = (list: Partition[]) => list.reduce((a, b) => (b.size > a.size ? b : a));

  // --- Alongside -------------------------------------------------------------

  interface ShrinkInfo {
    minBytes: number | null;
    error: string | null;
  }
  let probes: Record<string, ShrinkInfo | "probing"> = {};

  async function chooseTarget(path: string) {
    setDisk({ shrink: { partition: path, newSize: 0 } });
    const part = disk?.partitions.find((p) => p.path === path);
    if (!part || probes[path]) return;
    probes = { ...probes, [path]: "probing" };
    const info = await api
      .invoke<ShrinkInfo>("partition_min_size", { path, fstype: part.fstype })
      .catch(() => ({ minBytes: null, error: "nosudo" }));
    probes = { ...probes, [path]: info };
  }

  $: target = disk?.partitions.find((p) => p.path === $plan.disk.shrink?.partition);
  $: probe = target ? probes[target.path] : undefined;
  // The other system keeps what it uses plus a margin to go on working:
  // 10 % and at least 2 GiB.
  $: keepMin =
    target && probe && probe !== "probing" && probe.minBytes != null
      ? Math.min(alignUp(probe.minBytes + Math.max(probe.minBytes * 0.1, 2 * GiB)), target.size)
      : 0;
  $: keepMax = target && disk ? alignDown(target.size + spaceAfter(disk, target) - alongsideNeed(disk, options)) : 0;
  $: shrinkable = keepMin > 0 && keepMax >= keepMin;

  // A first proposal: half of what can be given, for each side.
  $: if ($plan.disk.mode === "alongside" && shrinkable && $plan.disk.shrink && !$plan.disk.shrink.newSize) {
    setDisk({ shrink: { ...$plan.disk.shrink, newSize: alignDown((keepMin + keepMax) / 2) } });
  }
  $: if ($plan.disk.shrink && shrinkable && $plan.disk.shrink.newSize) {
    const clamped = Math.min(Math.max($plan.disk.shrink.newSize, keepMin), keepMax);
    if (clamped !== $plan.disk.shrink.newSize) setDisk({ shrink: { ...$plan.disk.shrink, newSize: clamped } });
  }

  function onSlide(e: Event) {
    const value = Number((e.currentTarget as HTMLInputElement).value);
    if ($plan.disk.shrink) setDisk({ shrink: { ...$plan.disk.shrink, newSize: alignDown(value) } });
  }

  $: targetName = (p: Partition) =>
    `${p.path} · ${p.fstype.toUpperCase()}${p.label ? ` · ${p.label}` : ""} · ${size(p.size)}`;

  // --- The resulting table ---------------------------------------------------

  // The manual table as the user edits it, kept while going back and forth.
  // The plan gets it with the encryption flags applied.
  let raw: Layout | null = get(manualDraft);
  $: manualDraft.set(raw);

  $: computed = (() => {
    if (!disk) return null;
    const mode = $plan.disk.mode;
    if (mode === "erase") return eraseLayout(disk, options);
    if (mode === "alongside")
      return target && shrinkable && $plan.disk.shrink?.newSize
        ? alongsideLayout(disk, target, $plan.disk.shrink.newSize, options)
        : null;
    if (mode === "manual" && raw) {
      // The encryption checkbox applies to / and /home when they are written.
      const parts = raw.parts.map((p) => ({
        ...p,
        encrypt: $plan.disk.encrypt && ["/", "/home"].includes(p.mount) && ["create", "format"].includes(p.action),
      }));
      return { ...raw, parts };
    }
    return null;
  })();

  $: if (JSON.stringify(computed) !== JSON.stringify($plan.disk.layout)) {
    setDisk({ layout: computed });
  }

  // Next to another system with a swap, that swap is shared: one for every
  // system of the machine.
  $: diskSwap = $plan.disk.mode === "alongside" && disk ? existingSwap(disk) : undefined;

  $: problems =
    $plan.disk.mode === "manual" && computed && disk
      ? manualProblems(computed, { firmware: $plan.firmware, bootloader: $plan.bootloader, minRoot }, disk.table)
      : [];

  let confirmPassphrase = "";
  $: passphraseOk = !$plan.disk.encrypt || ($plan.disk.passphrase.length > 0 && $plan.disk.passphrase === confirmPassphrase);

  $: canContinue.set(!!disk && !!computed && problems.length === 0 && passphraseOk);

  /** Partitions that disappear with this layout. */
  $: deleted =
    disk && computed
      ? disk.partitions.filter((p) => computed!.newTable || !computed!.parts.some((q) => q.path === p.path))
      : [];

  function actionName(p: Planned) {
    return $_(`disk.actions.${p.action}`);
  }

  // Manual: the partitions this boot loader needs, ticked as they are made,
  // instead of complaining about each missing one.
  const MISSING = ["noRoot", "noEsp", "noBiosGrub", "noLimineFat"];
  $: required = [
    ...($plan.firmware === "bios" ? [] : [{ name: "EFI", ok: !problems.includes("noEsp") }]),
    ...($plan.firmware === "bios" && (computed?.newTable ? computed.table !== "msdos" : disk?.table === "gpt")
      ? [{ name: $_("disk.roles.biosgrub"), ok: !problems.includes("noBiosGrub") }]
      : []),
    ...($plan.firmware === "bios" && $plan.bootloader === "limine"
      ? [{ name: "/boot/limine", ok: !problems.includes("noLimineFat") }]
      : []),
    { name: "/", ok: !problems.includes("noRoot") },
  ];

  // The messages under the result, most serious first. One line is shown;
  // the rest are counted and listed in its tooltip.
  $: messages = [
    ...(!computed && $plan.disk.mode === "erase" ? [{ tone: "danger", text: $_("disk.problems.noRoomForSwap") }] : []),
    ...problems.filter((p) => !MISSING.includes(p)).map((problem) => ({ tone: "danger", text: $_(`disk.problems.${problem}`, { values: { size: size(minRoot) } }) })),
    ...(computed && deleted.length
      ? [
          {
            tone: "danger",
            text: computed.newTable && deleted.length === disk?.partitions.length
              ? $_("disk.wipesAll", { values: { disk: disk?.model || disk?.path || "" } })
              : $_("disk.deletes", { values: { list: deleted.map((p) => p.path).join(", ") } }),
          },
        ]
      : []),
    ...($plan.disk.mode === "alongside" && computed ? [{ tone: "warning", text: $_("disk.backupNote") }] : []),
  ];
  $: message = messages.length
    ? { ...messages[0], more: messages.length - 1, all: messages.map((m) => m.text).join("\n") }
    : null;

  /** "EFI", "NTFS", "ext4"... the name a partition is known by. */
  const fsName = (fs: string, esp = false) => (esp ? "EFI" : fsLabel(fs));

  $: beforeItems = disk
    ? disk.partitions.map((p) => ({
        start: p.start,
        size: p.size,
        color: fsColor(p.fstype),
        name: fsName(p.fstype, isEsp(p) && p.fstype === "vfat"),
        detail: `${size(p.size)} · ${p.path.replace("/dev/", "")}`,
        note: [p.path.replace("/dev/", ""), p.label || p.partlabel].filter(Boolean).join(" · "),
        faded: computed ? deleted.some((d) => d.path === p.path) : false,
      }))
    : [];

  function afterName(p: Planned) {
    if (p.parttype === GUID.biosGrub) return $_("disk.roles.biosgrub");
    if (p.mount === "/boot/efi") return "EFI";
    if (p.mount) return p.mount;
    if (p.fs === "swap") return "swap";
    return fsName(p.fs);
  }

  $: afterItems = computed
    ? computed.parts.map((p) => ({
        start: p.start,
        size: p.size,
        color: fsColor(p.fs),
        name: afterName(p),
        detail: size(p.size),
        note: [
          p.mount && p.fs ? fsLabel(p.fs) : "",
          p.encrypt ? "LUKS" : "",
          p.path ? p.path.replace("/dev/", "") : "",
          actionName(p),
        ]
          .filter(Boolean)
          .join(" · "),
        faded: p.action === "keep",
      }))
    : [];
</script>

<!-- The step has a fixed height: the disk (drawn once, as it is or as it
     will be) and the modes stay on top; only the middle scrolls. Adding
     partitions never moves anything else. -->
<div class="h-full min-h-0 flex flex-col gap-3">
  <!-- The disk -->
  <div class="shrink-0 rounded-lg border border-border bg-surface p-4 flex flex-col gap-3">
    {#if !loaded}
      <LoaderCircle size={20} class="animate-spin text-text-muted" />
    {:else if !disk}
      <p class="text-[0.85rem] text-danger">{$_("disk.noDisks")}</p>
    {:else}
      <div class="flex items-center gap-3.5">
        <span class="grid place-items-center w-10 h-10 rounded-lg bg-primary/15 text-primary shrink-0">
          <HardDrive size={20} />
        </span>
        <div class="min-w-0 flex-1">
          <p class="text-[0.95rem] font-semibold text-text-bright truncate">{disk.model || disk.path}</p>
          <p class="text-[0.78rem] text-text-muted">
            {[size(disk.size), disk.transport.toUpperCase(), disk.path, disk.table ? disk.table.replace("dos", "MBR").toUpperCase() : $_("disk.noTable")]
              .filter(Boolean)
              .join(" · ")}
          </p>
        </div>
        {#if disks.length > 1}
          <div class="w-72 shrink-0">
            <Select label={$_("disk.disk")} value={$plan.disk.device} options={diskOptions} onChange={selectDisk} />
          </div>
        {/if}
      </div>
      <!-- One drawing of the disk: as it is until a mode is chosen, then as
           it will be. -->
      {#if $plan.disk.mode ? computed : disk.partitions.length}
        <PartitionBar size={disk.size} items={$plan.disk.mode ? afterItems : beforeItems} legend={false} />
      {:else}
        <div class="h-14 rounded-lg free-space grid place-items-center text-[0.8rem] text-text-muted">
          {$plan.disk.mode ? "" : $_("disk.empty")}
        </div>
      {/if}
      <!-- One fixed line: it never pushes the page, and never scrolls. -->
      <div class="flex gap-x-4 h-5 overflow-hidden text-[0.78rem]">
        {#if $plan.disk.mode === "manual" && !afterItems.length}
          <span class="text-text-muted">{$_("disk.manualHint")}</span>
        {/if}
        {#each $plan.disk.mode ? afterItems : beforeItems as item}
          <span class="flex items-center gap-1.5 whitespace-nowrap" title={item.note ?? ""}>
            <span class="w-2.5 h-2.5 rounded-sm {item.faded ? 'opacity-45' : ''}" style="background: {item.color}"></span>
            <span class="text-text-bright font-medium">{item.name}</span>
            <span class="text-text-muted">{item.detail}</span>
          </span>
        {/each}
      </div>
      {#if $plan.disk.mode}
        <p class="flex items-center gap-2 h-5 text-[0.8rem] {message?.tone === 'danger' ? 'text-danger' : 'text-warning'}" title={message?.all ?? ""}>
          {#if $plan.disk.mode === "manual"}
            <span class="shrink-0 text-text-muted">{$_("disk.required")}</span>
            {#each required as r}
              <span class="shrink-0 flex items-center gap-1 {r.ok ? 'text-success' : 'text-text-muted'}">
                {#if r.ok}<CircleCheck size={14} />{:else}<Circle size={14} />{/if}
                {r.name}
              </span>
            {/each}
            {#if message}<span class="shrink-0 w-px h-3.5 bg-border mx-1"></span>{/if}
          {/if}
          {#if message}
            <TriangleAlert size={15} class="shrink-0" />
            <span class="truncate">{message.text}</span>
            {#if message.more > 0}<span class="shrink-0 text-text-muted">(+{message.more})</span>{/if}
          {/if}
        </p>
      {/if}
    {/if}
  </div>

  {#if disk}
    <!-- How -->
    <div class="shrink-0 grid grid-cols-3 gap-3" role="radiogroup" aria-label={$_("disk.mode")}>
      {#each MODES as m (m.id)}
        {@const selected = $plan.disk.mode === m.id}
        {@const enabled = available(m.id)}
        <button
          type="button"
          role="radio"
          aria-checked={selected}
          disabled={!enabled}
          on:click={() => chooseMode(m.id)}
          class="relative flex items-start gap-3 rounded-lg border-2 p-3 text-left transition-colors outline-none focus-visible:ring-2 focus-visible:ring-primary/60
            {selected ? 'border-primary bg-primary/10' : 'border-border bg-surface hover:border-primary/50'}
            {enabled ? '' : 'opacity-45 cursor-not-allowed hover:border-border'}"
        >
          <span class="grid place-items-center w-8 h-8 rounded-lg shrink-0 {selected ? 'bg-primary text-white' : 'bg-surface-2 text-primary'}">
            <svelte:component this={m.icon} size={17} />
          </span>
          <span class="min-w-0">
            <span class="block text-[0.9rem] font-semibold text-text-bright">{$_(`disk.modes.${m.id}`)}</span>
            <span class="block text-[0.78rem] text-text leading-snug mt-0.5">
              {enabled ? $_(`disk.modes.${m.id}Detail`) : $_(`disk.modes.${m.id}Unavailable`)}
            </span>
          </span>
          {#if selected}
            <span class="absolute top-2 right-2 grid place-items-center w-4 h-4 rounded-full bg-primary text-white">
              <Check size={11} strokeWidth={3} />
            </span>
          {/if}
        </button>
      {/each}
    </div>

    <!-- The middle: the only part that scrolls -->
    <div class="flex-1 min-h-0 flex flex-col gap-3">
      {#if $plan.disk.mode === "manual" && raw}
        <div class="flex-1 min-h-0">
          <ManualPartitioner {disk} layout={raw} firmware={$plan.firmware} bootloader={$plan.bootloader} onChange={(layout) => (raw = layout)} />
        </div>
      {:else if $plan.disk.mode}
        <div class="flex-1 min-h-0 overflow-y-auto stable-scroll flex flex-col gap-3">
          {#if $plan.disk.mode === "alongside"}
            <div class="rounded-lg border border-border bg-surface p-4 flex flex-col gap-3">
              <div class="grid grid-cols-[9rem_minmax(0,1fr)] items-center gap-2">
                <span class="text-[0.82rem] text-text">{$_("disk.shrinkWhich")}</span>
                <Select
                  label={$_("disk.shrinkWhich")}
                  value={$plan.disk.shrink?.partition ?? ""}
                  options={candidates.map((p) => ({ value: p.path, label: targetName(p) }))}
                  onChange={chooseTarget}
                />
              </div>
              {#if probe === "probing" || probe === undefined}
                <p class="flex items-center gap-2 text-[0.82rem] text-text-muted">
                  <LoaderCircle size={14} class="animate-spin" />
                  {$_("disk.measuring")}
                </p>
              {:else if probe.error}
                <p class="flex items-start gap-2 text-[0.82rem] text-danger">
                  <TriangleAlert size={15} class="shrink-0 mt-0.5" />
                  {$_(`disk.shrinkErrors.${probe.error}`, { default: $_("disk.shrinkErrors.unreadable") })}
                </p>
              {:else if !shrinkable}
                <p class="flex items-start gap-2 text-[0.82rem] text-danger">
                  <TriangleAlert size={15} class="shrink-0 mt-0.5" />
                  {$_("disk.shrinkErrors.full")}
                </p>
              {:else if $plan.disk.shrink && target}
                <input
                  type="range"
                  class="w-full accent-primary"
                  min={keepMin}
                  max={keepMax}
                  step={MiB * 256}
                  value={$plan.disk.shrink.newSize}
                  on:input={onSlide}
                  aria-label={$_("disk.shrinkWhich")}
                />
                <div class="flex justify-between text-[0.82rem]">
                  <span class="text-text">
                    <span class="inline-block w-2.5 h-2.5 rounded-sm mr-1.5 align-middle" style="background: {fsColor(target.fstype)}"></span>
                    {$_("disk.keepsOther", { values: { size: size($plan.disk.shrink.newSize) } })}
                  </span>
                  <span class="text-text">
                    <span class="inline-block w-2.5 h-2.5 rounded-sm mr-1.5 align-middle" style="background: {fsColor($plan.disk.filesystem)}"></span>
                    {$_("disk.forSoplos", {
                      values: { size: size(target.size + spaceAfter(disk, target) - $plan.disk.shrink.newSize) },
                    })}
                  </span>
                </div>
              {/if}
            </div>
          {/if}
          <div class="rounded-lg border border-border bg-surface p-4 grid grid-cols-2 gap-x-6 gap-y-3">
            <div class="grid grid-cols-[7.5rem_minmax(0,1fr)] items-center gap-2">
              <span class="text-[0.82rem] text-text">{$_("disk.filesystem")}</span>
              <Select
                label={$_("disk.filesystem")}
                value={$plan.disk.filesystem}
                options={[
                  { value: "btrfs", label: `btrfs — ${$_("disk.btrfsNote")}` },
                  { value: "ext4", label: "ext4" },
                  { value: "xfs", label: "xfs" },
                ]}
                onChange={setFilesystem}
              />
            </div>
            <!-- Next to another system that has a swap, that swap is shared and
                 there is nothing to choose (see alongsideExtras). -->
            {#if diskSwap}
              <div class="grid grid-cols-[7.5rem_minmax(0,1fr)] items-center gap-2">
                <span class="text-[0.82rem] text-text">{$_("disk.swap")}</span>
                <span class="text-[0.88rem] text-text-bright truncate">{$_("summary.swapShared")} · {size(diskSwap.size)}</span>
              </div>
            {:else}
              <div class="grid grid-cols-[7.5rem_minmax(0,1fr)] items-center gap-2">
                <span class="text-[0.82rem] text-text">{$_("disk.swap")}</span>
                <Select
                  label={$_("disk.swap")}
                  value={$plan.disk.swap}
                  options={[
                    { value: "none", label: $_("disk.swapNone") },
                    { value: "swap", label: $_("disk.swapNormal") },
                    { value: "hibernate", label: $_("disk.swapHibernate", { values: { size: size(swapSize("hibernate", ramBytes)) } }) },
                  ]}
                  onChange={setSwap}
                />
              </div>
            {/if}
          </div>
        </div>
      {:else}
        <div class="flex-1"></div>
      {/if}

      {#if $plan.disk.mode}
        <!-- Encryption: one fixed row -->
        <div class="shrink-0 rounded-lg border border-border bg-surface px-4 py-2.5 flex items-center gap-3 h-[3.25rem]">
          <label class="flex items-center gap-2.5 cursor-pointer shrink-0">
            <input
              type="checkbox"
              class="w-4 h-4 accent-primary"
              checked={$plan.disk.encrypt}
              on:change={(e) => setDisk({ encrypt: e.currentTarget.checked, passphrase: "" })}
            />
            <Lock size={15} class="text-primary" />
            <span class="text-[0.85rem] text-text-bright">{$_("disk.encrypt")}</span>
          </label>
          {#if $plan.disk.encrypt}
            <input
              type="password"
              autocomplete="new-password"
              placeholder={$_("disk.passphrase")}
              value={$plan.disk.passphrase}
              on:input={(e) => setDisk({ passphrase: e.currentTarget.value })}
              class="flex-1 min-w-0 bg-surface-2 border border-border rounded px-3 py-1.5 text-[0.85rem] text-text-bright placeholder:text-text-muted outline-none focus-visible:ring-2 focus-visible:ring-primary/60"
            />
            <input
              type="password"
              autocomplete="new-password"
              placeholder={$_("disk.passphraseConfirm")}
              bind:value={confirmPassphrase}
              class="flex-1 min-w-0 bg-surface-2 border rounded px-3 py-1.5 text-[0.85rem] text-text-bright placeholder:text-text-muted outline-none focus-visible:ring-2 focus-visible:ring-primary/60
                {confirmPassphrase && !passphraseOk ? 'border-danger' : 'border-border'}"
            />
          {:else}
            <span class="text-[0.78rem] text-text-muted truncate">{$_("disk.encryptNote")}</span>
          {/if}
        </div>
      {/if}
    </div>

  {/if}
</div>

<style>
  .free-space {
    background: repeating-linear-gradient(
      135deg,
      rgb(var(--color-surface-2)),
      rgb(var(--color-surface-2)) 6px,
      rgb(var(--color-dark)) 6px,
      rgb(var(--color-dark)) 12px
    );
  }
  .stable-scroll {
    scrollbar-gutter: stable;
  }
</style>

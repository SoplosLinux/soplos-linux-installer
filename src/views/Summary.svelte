<script lang="ts">
  import { onMount } from "svelte";
  import { _, locale } from "svelte-i18n";
  import { Globe, Power, HardDrive, UserRound, Pencil, TriangleAlert } from "lucide-svelte";
  import type { ComponentType } from "svelte";
  import { api } from "../lib/api";
  import { plan, current, canContinue, STEPS, type StepId } from "../lib/wizard";
  import { cityOf, countryName, localeName, type Layout as KeyboardLayout, type Zone } from "../lib/location";
  import { type Disk, type BarItem, GUID, fsColor, fsLabel, formatSize } from "../lib/partitioning";
  import PartitionBar from "../components/PartitionBar.svelte";
  import { kernelName } from "../lib/kernels";

  // DESIGN.md, section 4.6: everything chosen, and what will be done to the
  // disk, before the last confirmation. Each block leads back to its step.

  let layouts: KeyboardLayout[] = [];
  let zones: Zone[] = [];
  let disks: Disk[] = [];

  $: ui = $locale?.split("-")[0] ?? "en";
  $: size = (bytes: number) => formatSize(bytes, $locale ?? "en");

  onMount(async () => {
    const [l, data, d] = await Promise.all([
      api.invoke<KeyboardLayout[]>("get_keyboard_layouts", { language: ui }).catch(() => [] as KeyboardLayout[]),
      api.invoke<{ zones: Zone[] }>("get_location_data").catch(() => ({ zones: [] as Zone[] })),
      api.invoke<Disk[]>("list_disks").catch(() => [] as Disk[]),
    ]);
    layouts = l;
    zones = data.zones;
    disks = d;
  });

  $: canContinue.set(!!$plan.disk.layout);

  function edit(step: StepId) {
    canContinue.set(true);
    current.set(STEPS.indexOf(step));
  }

  $: zone = zones.find((z) => z.id === $plan.timezone);
  $: layout = layouts.find((l) => l.name === $plan.keyboard.layout);
  $: variant = layout?.variants.find((v) => v.name === $plan.keyboard.variant);
  $: disk = disks.find((d) => d.path === $plan.disk.device);
  $: parts = $plan.disk.layout?.parts ?? [];

  const BOOTLOADER: Record<string, string> = { grub: "GRUB", "systemd-boot": "systemd-boot", limine: "Limine" };

  function partName(p: (typeof parts)[number]) {
    if (p.parttype === GUID.biosGrub) return $_("disk.roles.biosgrub");
    if (p.mount === "/boot/efi") return "EFI";
    return p.mount || (p.fs === "swap" ? "swap" : fsLabel(p.fs));
  }

  $: barItems = parts.map(
    (p): BarItem => ({
      start: p.start,
      size: p.size,
      color: fsColor(p.fs),
      name: partName(p),
      detail: size(p.size),
      faded: p.action === "keep",
    }),
  );

  // What happens to the disk, in plain words: the part that cannot be undone.
  $: changes = (() => {
    if (!disk || !$plan.disk.layout) return [] as string[];
    const out: string[] = [];
    const l = $plan.disk.layout;
    const removed = disk.partitions.filter((p) => l.newTable || !l.parts.some((q) => q.path === p.path));
    if (l.newTable && removed.length === disk.partitions.length && removed.length)
      out.push($_("summary.wipes", { values: { disk: disk.model || disk.path } }));
    else if (removed.length) out.push($_("disk.deletes", { values: { list: removed.map((p) => p.path).join(", ") } }));
    for (const p of l.parts) {
      if (p.action === "shrink") out.push($_("summary.shrinks", { values: { part: p.path, size: size(p.size) } }));
      if (p.action === "format") out.push($_("summary.formats", { values: { part: p.path, fs: fsLabel(p.fs) } }));
    }
    const created = l.parts.filter((p) => p.action === "create").length;
    if (created) out.push($_("summary.creates", { values: { count: created } }));
    return out;
  })();

  $: swapText =
    $plan.disk.mode === "manual"
      ? parts.some((p) => p.fs === "swap")
        ? $_("summary.yes")
        : $_("disk.swapNone")
      : $plan.disk.swap === "none"
        ? $_("disk.swapNone")
        : $plan.disk.swap === "hibernate"
          ? $_("summary.swapHibernate")
          : $_("summary.swapPlain");

  interface Block {
    step: StepId;
    icon: ComponentType;
    title: string;
    rows: [string, string][];
  }

  $: blocks = [
    {
      step: "location",
      icon: Globe,
      title: $_("steps.location.title"),
      rows: [
        [$_("location.zone"), zone ? `${cityOf(zone.id)} · ${countryName(zone.country, ui)}` : $plan.timezone],
        [$_("location.languageTitle"), $plan.locale ? localeName($plan.locale, ui) : ""],
        [$_("location.formats"), $plan.formats ? localeName($plan.formats, ui) : ""],
        [$_("location.keyboard"), [layout?.description ?? $plan.keyboard.layout, variant?.description].filter(Boolean).join(" · ")],
      ],
    },
    {
      step: "user",
      icon: UserRound,
      title: $_("steps.user.title"),
      rows: [
        [$_("user.fullName"), $plan.user.fullName],
        [$_("user.login"), $plan.user.login],
        [$_("user.hostname"), $plan.user.hostname],
        [$_("summary.root"), $_($plan.user.sameRootPassword ? "summary.rootSame" : "summary.rootOwn")],
      ],
    },
    {
      step: "bootloader",
      icon: Power,
      title: `${$_("steps.bootloader.title")} · ${$_("steps.kernel.title")}`,
      rows: [
        [$_("summary.firmware"), $plan.firmware === "bios" ? "BIOS (legacy)" : "UEFI"],
        [$_("summary.bootloader"), BOOTLOADER[$plan.bootloader] ?? ""],
        [$_("summary.installation"), $_(`kernel.modes.${$plan.kernel.mode}`)],
        [$_("summary.kernel"), $plan.kernel.mode === "minimal" ? $_("summary.imageKernel") : kernelName($plan.kernel.flavor)],
      ],
    },
    {
      step: "disk",
      icon: HardDrive,
      title: $_("steps.disk.title"),
      rows: [
        [$_("disk.disk"), disk ? `${disk.model || disk.path} · ${size(disk.size)} · ${disk.path}` : $plan.disk.device],
        [$_("disk.mode"), $plan.disk.mode ? $_(`disk.modes.${$plan.disk.mode}`) : ""],
        ...($plan.disk.mode !== "manual" ? [[$_("disk.filesystem"), $plan.disk.filesystem] as [string, string]] : []),
        [$_("disk.swap"), swapText],
        [$_("summary.encryption"), $_($plan.disk.encrypt ? "summary.yes" : "summary.no")],
      ],
    },
  ] satisfies Block[];
</script>

<div class="flex flex-col gap-4">
  <div class="grid grid-cols-2 gap-4">
    {#each blocks as block (block.step)}
      <div class="rounded-lg border border-border bg-surface p-4 min-w-0">
        <div class="flex items-center gap-2 mb-2.5">
          <svelte:component this={block.icon} size={17} class="text-primary shrink-0" />
          <p class="text-[0.88rem] text-text-bright font-semibold flex-1">{block.title}</p>
          <button
            type="button"
            class="flex items-center gap-1 rounded px-2 py-0.5 text-[0.75rem] text-text-muted hover:bg-surface-2 hover:text-primary"
            on:click={() => edit(block.step)}
          >
            <Pencil size={12} />
            {$_("summary.change")}
          </button>
        </div>
        <dl class="grid grid-cols-[minmax(0,9.5rem)_minmax(0,1fr)] gap-x-3 gap-y-1 text-[0.82rem]">
          {#each block.rows as [label, value]}
            <dt class="text-text-muted truncate">{label}</dt>
            <dd class="text-text-bright truncate" title={value}>{value}</dd>
          {/each}
        </dl>
      </div>
    {/each}
  </div>

  {#if disk && barItems.length}
    <div class="rounded-lg border border-border bg-surface p-4 flex flex-col gap-2.5">
      <PartitionBar size={disk.size} items={barItems} legend={false} />
      <div class="flex gap-x-4 h-5 overflow-hidden text-[0.78rem]">
        {#each barItems as item}
          <span class="flex items-center gap-1.5 whitespace-nowrap">
            <span class="w-2.5 h-2.5 rounded-sm {item.faded ? 'opacity-45' : ''}" style="background: {item.color}"></span>
            <span class="text-text-bright font-medium">{item.name}</span>
            <span class="text-text-muted">{item.detail}</span>
          </span>
        {/each}
      </div>
      {#each changes as change}
        <p class="flex items-start gap-2 text-[0.82rem] text-warning">
          <TriangleAlert size={15} class="shrink-0 mt-0.5" />
          {change}
        </p>
      {/each}
    </div>
  {/if}
</div>

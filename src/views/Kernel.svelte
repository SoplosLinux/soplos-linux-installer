<script lang="ts">
  import { onMount } from "svelte";
  import { _ } from "svelte-i18n";
  import { Cpu, Box, Gamepad2, Wind, AudioWaveform, Layers, Check, Copy, Wrench, RefreshCw, LoaderCircle } from "lucide-svelte";
  import type { ComponentType } from "svelte";
  import { api } from "../lib/api";
  import { plan, canContinue, type InstallPlan } from "../lib/wizard";
  import { FAMILY_NAMES, isLts, familyOf, flavorFor, familyName } from "../lib/kernels";

  // DESIGN.md, section 4 (Kernel): the kind of installation, each one adding
  // to the one before (minimal: the image as it is; standard: drivers and
  // the kernel chosen; full: and the updates), and the Soplos kernels of the
  // repository for the level of the processor, as Soplos Kernel Installer
  // lists them, each in its latest or its LTS branch. The level is the
  // helper's (commands/kernels.rs).

  type Mode = InstallPlan["kernel"]["mode"];
  const MODES: { id: Mode; icon: ComponentType }[] = [
    { id: "minimal", icon: Copy },
    { id: "standard", icon: Wrench },
    { id: "full", icon: RefreshCw },
  ];
  function setMode(mode: Mode) {
    plan.update((p) => ({ ...p, kernel: { ...p.kernel, mode } }));
  }
  $: minimal = $plan.kernel.mode === "minimal";

  interface KernelOption {
    flavor: string;
    package: string;
    version: string;
  }

  let level = 0;
  let kernels: KernelOption[] = [];
  let loading = true;

  onMount(async () => {
    const list = await api
      .invoke<{ level: number; kernels: KernelOption[] }>("get_kernels")
      .catch(() => ({ level: 0, kernels: [] as KernelOption[] }));
    level = list.level;
    kernels = list.kernels;
    loading = false;
    // A kernel chosen before that the repository no longer has: stock.
    if (!kernels.some((k) => k.flavor === $plan.kernel.flavor)) setFlavor("");
  });

  canContinue.set(true);

  function setFlavor(flavor: string) {
    plan.update((p) => ({ ...p, kernel: { ...p.kernel, flavor } }));
  }

  // The icon of each family Soplos builds; any other the repository adds
  // goes after them with a generic icon and text.
  const ICONS: Record<string, ComponentType> = { "": Box, "bore-ntsync": Gamepad2, zen: Wind, rt: AudioWaveform, x3d: Cpu };
  const ORDER = Object.keys(FAMILY_NAMES);
  const known = (f: string) => f in FAMILY_NAMES;

  $: lts = isLts($plan.kernel.flavor);
  $: family = familyOf($plan.kernel.flavor);
  $: hasLts = kernels.some((k) => isLts(k.flavor));
  $: families = [...new Set(kernels.map((k) => familyOf(k.flavor)))].sort((a, b) => {
    const ia = ORDER.indexOf(a);
    const ib = ORDER.indexOf(b);
    return (ia < 0 ? 99 : ia) - (ib < 0 ? 99 : ib) || a.localeCompare(b);
  });
  $: find = (f: string, branch: boolean) => kernels.find((k) => k.flavor === flavorFor(f, branch));

  function chooseFamily(f: string) {
    if (find(f, lts)) setFlavor(flavorFor(f, lts));
  }

  // Changing the branch keeps the family when it has that branch, and goes
  // to stock when it does not.
  function chooseBranch(branch: boolean) {
    setFlavor(find(family, branch) ? flavorFor(family, branch) : flavorFor("", branch));
  }

  // "6.19.2-1soplos1" -> "6.19.2"
  const shortVersion = (v: string) => v.match(/^\d+\.\d+(\.\d+)?/)?.[0] ?? v;
</script>

<div class="flex flex-col gap-3.5">
  <div class="grid grid-cols-3 gap-3" role="radiogroup" aria-label={$_("kernel.mode")}>
    {#each MODES as m (m.id)}
      {@const selected = $plan.kernel.mode === m.id}
      <button
        type="button"
        role="radio"
        aria-checked={selected}
        on:click={() => setMode(m.id)}
        class="relative flex items-start gap-2.5 rounded-lg border-2 px-3.5 py-2.5 text-left transition-colors outline-none focus-visible:ring-2 focus-visible:ring-primary/60
          {selected ? 'border-primary bg-primary/10' : 'border-border bg-surface hover:border-primary/50'}"
      >
        <span class="grid place-items-center w-8 h-8 rounded-lg shrink-0 {selected ? 'bg-primary text-white' : 'bg-surface-2 text-primary'}">
          <svelte:component this={m.icon} size={17} />
        </span>
        <span class="min-w-0">
          <span class="flex items-center gap-1.5 text-[0.92rem] font-semibold text-text-bright leading-tight">
            {$_(`kernel.modes.${m.id}`)}
            {#if m.id === "full"}
              <span class="rounded-full bg-primary/15 text-primary px-1.5 py-px text-[0.68rem] font-semibold">{$_("bootloader.recommended")}</span>
            {/if}
          </span>
          <span class="block text-[0.76rem] text-text leading-snug mt-0.5">{$_(`kernel.modes.${m.id}Detail`)}</span>
        </span>
      </button>
    {/each}
  </div>

  <div class="flex items-center gap-3 rounded-lg border border-border bg-surface px-4 h-12">
    <Cpu size={18} class="text-primary shrink-0" />
    <p class="min-w-0 flex-1 text-[0.85rem] text-text truncate">
      <span class="text-text-bright font-semibold">
        {level ? $_("kernel.level", { values: { level: `x86-64-v${level}` } }) : $_("kernel.levelUnknown")}
      </span>
      · {$_(minimal ? "kernel.minimalNote" : "kernel.levelDetail")}
    </p>
    {#if hasLts}
      <div class="flex shrink-0 rounded-lg border border-border bg-surface-2 p-0.5 {minimal ? 'opacity-45 pointer-events-none' : ''}" role="radiogroup" aria-label={$_("kernel.branch")}>
        {#each [false, true] as branch}
          <button
            type="button"
            role="radio"
            aria-checked={lts === branch}
            disabled={minimal}
            on:click={() => chooseBranch(branch)}
            class="rounded-md px-3 py-0.5 text-[0.8rem] font-semibold transition-colors outline-none focus-visible:ring-2 focus-visible:ring-primary/60
              {lts === branch ? 'bg-primary text-white' : 'text-text hover:text-text-bright'}"
          >
            {$_(branch ? "kernel.lts" : "kernel.latest")}
          </button>
        {/each}
      </div>
    {/if}
  </div>

  <!-- A fixed height for the cards, so the step does not change size while
       the repository is read or when it has fewer kernels. -->
  <div class="h-[15rem] {minimal ? 'opacity-45 pointer-events-none' : ''}">
    {#if loading}
      <div class="h-full flex items-center justify-center gap-2.5 rounded-lg border border-border bg-surface text-[0.85rem] text-text">
        <LoaderCircle size={18} class="animate-spin text-primary" />
        {$_("kernel.loading")}
      </div>
    {:else if !kernels.length}
      <div class="h-full flex items-center justify-center gap-3 rounded-lg border border-border bg-surface p-6">
        <Layers size={20} class="text-warning shrink-0" />
        <p class="text-[0.85rem] text-text max-w-xl">{$_("kernel.none")}</p>
      </div>
    {:else}
      <div class="grid grid-cols-3 grid-rows-2 gap-3 h-full" role="radiogroup" aria-label={$_("steps.kernel.title")}>
        {#each families as f (f)}
          {@const option = find(f, lts)}
          {@const selected = !minimal && family === f && !!option}
          <button
            type="button"
            role="radio"
            aria-checked={selected}
            disabled={!option || minimal}
            on:click={() => chooseFamily(f)}
            class="relative flex flex-col gap-1.5 rounded-lg border-2 px-3.5 py-2.5 text-left transition-colors outline-none focus-visible:ring-2 focus-visible:ring-primary/60 min-h-0 overflow-hidden
              {selected ? 'border-primary bg-primary/10' : 'border-border bg-surface hover:border-primary/50'}
              {option ? '' : 'opacity-45 cursor-not-allowed hover:border-border'}"
          >
            {#if selected}
              <span class="absolute top-2.5 right-2.5 grid place-items-center w-5 h-5 rounded-full bg-primary text-white">
                <Check size={13} strokeWidth={3} />
              </span>
            {/if}
            <span class="flex items-center gap-2.5">
              <span class="grid place-items-center w-8 h-8 rounded-lg shrink-0 {selected ? 'bg-primary text-white' : 'bg-surface-2 text-primary'}">
                <svelte:component this={ICONS[f] ?? Layers} size={17} />
              </span>
              <span class="min-w-0">
                <span class="flex items-center gap-1.5 text-[0.92rem] font-semibold text-text-bright leading-tight">
                  <span class="truncate">{familyName(f)}</span>
                  {#if f === ""}
                    <span class="rounded-full bg-primary/15 text-primary px-1.5 py-px text-[0.68rem] font-semibold shrink-0">{$_("bootloader.recommended")}</span>
                  {/if}
                </span>
                <span class="block text-[0.74rem] text-text-muted">
                  {option ? shortVersion(option.version) : $_("kernel.noBranch")}
                </span>
              </span>
            </span>
            <span class="text-[0.78rem] text-text leading-snug line-clamp-3">
              {$_(known(f) ? `kernel.flavors.${f || "stock"}` : "kernel.flavors.other")}
            </span>
          </button>
        {/each}
      </div>
    {/if}
  </div>
</div>

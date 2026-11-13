<script lang="ts">
  import { _ } from "svelte-i18n";
  import { Cpu, ShieldCheck, Zap, Sparkles, Check, Info } from "lucide-svelte";
  import type { ComponentType } from "svelte";
  import { plan, canContinue, type InstallPlan } from "../lib/wizard";

  // DESIGN.md, section 4.3: UEFI offers the three, BIOS only GRUB and Limine.
  // All three can boot with Secure Boot: GRUB and systemd-boot behind shim
  // with the signatures of Debian, Limine signed with the user's own key
  // (MOK) from Soplos Kernel Installer.
  type Id = InstallPlan["bootloader"];
  interface Option {
    id: Exclude<Id, "">;
    key: string;
    name: string;
    icon: ComponentType;
    bios: boolean;
    /** How Secure Boot is kept: the signature of Debian or the user's MOK. */
    secureBoot: "debian" | "mok";
  }

  const OPTIONS: Option[] = [
    { id: "grub", key: "grub", name: "GRUB", icon: ShieldCheck, bios: true, secureBoot: "debian" },
    { id: "systemd-boot", key: "systemdBoot", name: "systemd-boot", icon: Zap, bios: false, secureBoot: "debian" },
    { id: "limine", key: "limine", name: "Limine", icon: Sparkles, bios: true, secureBoot: "mok" },
  ];

  $: uefi = $plan.firmware !== "bios";
  $: available = (o: Option) => uefi || o.bios;

  // A choice that this firmware cannot boot falls back to GRUB.
  $: if (!OPTIONS.some((o) => o.id === $plan.bootloader && available(o))) {
    plan.update((p) => ({ ...p, bootloader: "grub" }));
  }
  $: canContinue.set(!!$plan.bootloader);


  function choose(o: Option) {
    if (available(o)) plan.update((p) => ({ ...p, bootloader: o.id }));
  }
</script>

<div class="flex flex-col gap-4">
  <div class="flex items-center gap-3 rounded-lg border border-border bg-surface p-4">
    <Cpu size={20} class="text-primary shrink-0" />
    <div class="min-w-0">
      <p class="text-[0.9rem] text-text-bright font-semibold">
        {$_(uefi ? "bootloader.firmwareUefi" : "bootloader.firmwareBios")}
      </p>
      <p class="text-[0.82rem] text-text mt-0.5">
        {$_(uefi ? "bootloader.firmwareUefiDetail" : "bootloader.firmwareBiosDetail")}
      </p>
    </div>
  </div>

  <div class="grid grid-cols-3 gap-4" role="radiogroup" aria-label={$_("steps.bootloader.title")}>
    {#each OPTIONS as o (o.id)}
      {@const selected = $plan.bootloader === o.id}
      {@const enabled = available(o)}
      <button
        type="button"
        role="radio"
        aria-checked={selected}
        disabled={!enabled}
        on:click={() => choose(o)}
        class="relative flex flex-col gap-2.5 rounded-lg border-2 p-4 text-left transition-colors outline-none focus-visible:ring-2 focus-visible:ring-primary/60
          {selected ? 'border-primary bg-primary/10' : 'border-border bg-surface hover:border-primary/50'}
          {enabled ? '' : 'opacity-45 cursor-not-allowed hover:border-border'}"
      >
        {#if selected}
          <span class="absolute top-3 right-3 grid place-items-center w-5 h-5 rounded-full bg-primary text-white">
            <Check size={13} strokeWidth={3} />
          </span>
        {/if}
        <span class="grid place-items-center w-10 h-10 rounded-lg {selected ? 'bg-primary text-white' : 'bg-surface-2 text-primary'}">
          <svelte:component this={o.icon} size={20} />
        </span>
        <span class="text-[1rem] font-semibold text-text-bright">{o.name}</span>
        <span class="text-[0.82rem] text-text leading-snug flex-1">{$_(`bootloader.${o.key}`)}</span>
        <span class="flex flex-wrap gap-1.5">
          {#if o.id === "grub"}
            <span class="rounded-full bg-primary/15 text-primary px-2 py-0.5 text-[0.72rem] font-semibold">{$_("bootloader.recommended")}</span>
          {/if}
          <span class="rounded-full bg-success/15 text-success px-2 py-0.5 text-[0.72rem] font-semibold">
            {o.secureBoot === "mok" ? $_("bootloader.secureBootMok") : "Secure Boot"}
          </span>
          <span class="rounded-full bg-surface-2 text-text px-2 py-0.5 text-[0.72rem]">
            {$_(o.bios ? "bootloader.uefiAndBios" : "bootloader.onlyUefi")}
          </span>
        </span>
      </button>
    {/each}
  </div>

  <div class="flex items-start gap-3 rounded-lg border border-border bg-surface p-4">
    <Info size={18} class="text-primary shrink-0 mt-0.5" />
    <p class="text-[0.85rem] text-text">{$_("bootloader.secureBootNote")}</p>
  </div>
</div>

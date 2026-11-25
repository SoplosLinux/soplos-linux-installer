<script lang="ts">
  import type { ComponentType } from "svelte";
  import { getContext } from "svelte";
  import { theme, toggleTheme } from "../lib/theme";
  import { Sun, Moon, Check } from "lucide-svelte";
  import { _ } from "svelte-i18n";

  // The steps of the installer, one under the other. The one on screen is
  // highlighted, the ones already done carry a check and can be opened again;
  // the ones still ahead are only listed. Nothing can be opened once the
  // installation has started.
  export let steps: { id: string; icon: ComponentType; label: string }[];
  export let current: number;
  export let reached: number;
  export let locked = false;
  export let onSelect: (index: number) => void;
  export let appVersion = "";
  export let edition = "";

  const openAbout = getContext<() => void>("openAbout");

  const EDITION_LABELS: Record<string, string> = {
    tyron: "Tyron",
    tyson: "Tyson",
    boro: "Boro",
  };

  // The Debian revision is not shown: "3.0.0-1-alpha" -> "3.0.0-alpha",
  // "3.0.0-1" -> "3.0.0". About still shows the whole version.
  $: cleanVersion = appVersion.replace(/-\d+(?=-|$)/, "");
  $: footerLabel = [EDITION_LABELS[edition] ?? "", cleanVersion].filter(Boolean).join(" · ") || "Soplos Linux Installer";
</script>

<aside class="w-[230px] shrink-0 flex flex-col pt-[18px] px-[10px] pb-2 bg-panel border-r border-border" data-tauri-drag-region>
  <!-- With no frame of the system, the top of the sidebar moves the window. -->
  <div class="flex items-center gap-2 px-3 pb-[22px] font-bold text-[0.98rem] text-text-bright" data-tauri-drag-region="deep">
    <img src="/assets/icons/soplos-logo.png" alt="Soplos" class="w-[28px] h-[28px] object-contain shrink-0" />
    <span class="flex flex-col leading-tight min-w-0">
      <span class="whitespace-nowrap">Soplos</span>
      <span class="whitespace-nowrap">Linux Installer</span>
    </span>
  </div>

  <nav class="flex flex-col gap-[3px] flex-1">
    {#each steps as step, index}
      {@const done = index < current}
      {@const open = !locked && index <= reached && index !== current}
      <button
        class="flex items-center gap-3 px-3.5 py-2.5 rounded text-[0.92rem] text-left transition-colors
          {index === current
          ? 'bg-primary/15 text-primary font-semibold'
          : open
            ? 'text-text hover:bg-surface hover:text-text-bright'
            : 'text-text-muted cursor-default'}"
        disabled={!open}
        aria-current={index === current ? "step" : undefined}
        on:click={() => onSelect(index)}
      >
        {#if done}
          <Check size={18} class="shrink-0 text-success" />
        {:else}
          <svelte:component this={step.icon} size={18} class="shrink-0" />
        {/if}
        <span>{step.label}</span>
      </button>
    {/each}
  </nav>

  <div class="mt-3 pt-2.5 border-t border-border flex items-center gap-2 px-1">
    <button
      class="p-1 rounded text-text-muted hover:bg-surface hover:text-text-bright shrink-0"
      on:click={toggleTheme}
      title={$theme === "dark" ? $_("sidebar.switchToLight") : $_("sidebar.switchToDark")}
      aria-label={$theme === "dark" ? $_("sidebar.switchToLight") : $_("sidebar.switchToDark")}
    >
      <svelte:component this={$theme === "dark" ? Moon : Sun} size={14} />
    </button>
    <button
      class="text-[0.62rem] text-text-muted hover:text-text-bright hover:underline"
      on:click={openAbout}
      title={$_("sidebar.aboutTooltip")}
    >
      {footerLabel}
    </button>
  </div>
</aside>

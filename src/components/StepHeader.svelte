<script lang="ts">
  import type { ComponentType } from "svelte";

  // The banner over every step: the image of the installer, the icon of the
  // step, its title and one line that says what is decided there.
  export let title: string;
  export let subtitle = "";
  export let icon: ComponentType;
</script>

<div
  class="rounded px-5 py-6 mb-4 flex items-center gap-3.5 border border-border"
  style="background: linear-gradient(rgba(20, 20, 22, 0.55), rgba(20, 20, 22, 0.82)),
    url('/assets/hero/linux-installer-background.jpg'); background-size: cover; background-position: center;"
>
  <div class="w-[42px] h-[42px] rounded-lg grid place-items-center bg-white/15 text-white shrink-0">
    <svelte:component this={icon} size={22} />
  </div>
  <div class="min-w-0">
    <h2 class="text-[1.3rem] font-bold text-white leading-tight">{title}</h2>
    {#if subtitle}
      <p class="text-[0.85rem] text-white/85 mt-0.5">{subtitle}</p>
    {/if}
  </div>
</div>

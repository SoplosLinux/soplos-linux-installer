<script lang="ts">
  import { type BarItem, inkFor } from "../lib/partitioning";
  // A disk drawn to scale: each partition a rounded block where it lies, with
  // its name and size when it fits; free space striped. Tiny partitions keep
  // a sliver so they can be seen. The legend below lists them all.
  export let size: number;
  export let items: BarItem[] = [];
  export let legend = true;

  const pct = (x: number) => (size > 0 ? (x / size) * 100 : 0);
</script>

<div class="relative h-14 w-full rounded-lg free-space">
  {#each items as item}
    <div
      class="absolute inset-y-0 p-[2px]"
      style="left: {pct(item.start)}%; width: max(7px, {pct(item.size)}%);"
      title="{item.name} · {item.detail}{item.note ? ` · ${item.note}` : ''}"
    >
      <div
        class="h-full w-full rounded-md flex flex-col items-center justify-center overflow-hidden shadow-sm {item.faded ? 'opacity-45' : ''}"
        style="background: {item.color}; color: {inkFor(item.color)};"
      >
        {#if pct(item.size) > 7}
          <span class="max-w-full truncate px-2 text-[0.78rem] font-bold leading-tight">{item.name}</span>
          <span class="max-w-full truncate px-2 text-[0.7rem] opacity-75 leading-tight">{item.detail}</span>
        {/if}
      </div>
    </div>
  {/each}
</div>

{#if legend && items.length}
  <ul class="grid grid-cols-2 gap-x-6 gap-y-1.5 mt-3">
    {#each items as item}
      <li class="flex items-center gap-2 min-w-0 text-[0.8rem]">
        <span class="w-3 h-3 rounded shrink-0 {item.faded ? 'opacity-45' : ''}" style="background: {item.color}"></span>
        <span class="text-text-bright font-medium whitespace-nowrap">{item.name}</span>
        <span class="text-text-muted whitespace-nowrap">{item.detail}</span>
        {#if item.note}
          <span class="text-text-muted truncate">· {item.note}</span>
        {/if}
      </li>
    {/each}
  </ul>
{/if}

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
</style>

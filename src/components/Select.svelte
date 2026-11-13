<script lang="ts">
  import { ChevronDown } from "lucide-svelte";

  // A select drawn by the theme. appearance-none: WebKitGTK otherwise draws
  // the native widget of the system, which ignores the colors of the theme.
  export let value: string;
  export let options: { value: string; label: string }[] = [];
  export let label = "";
  export let onChange: (value: string) => void = () => {};

  function change(e: Event) {
    value = (e.currentTarget as HTMLSelectElement).value;
    onChange(value);
  }
</script>

<span class="relative block min-w-0">
  <select
    aria-label={label}
    class="w-full appearance-none truncate bg-surface-2 border border-border rounded pl-3 pr-8 py-1.5 text-[0.88rem] text-text-bright outline-none focus-visible:ring-2 focus-visible:ring-primary/60"
    {value}
    on:change={change}
  >
    {#each options as option (option.value)}
      <option value={option.value}>{option.label}</option>
    {/each}
  </select>
  <ChevronDown size={14} class="pointer-events-none absolute right-2.5 top-1/2 -translate-y-1/2 text-text-muted" />
</span>

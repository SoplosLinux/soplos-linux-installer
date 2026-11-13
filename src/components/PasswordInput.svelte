<script lang="ts">
  import { _ } from "svelte-i18n";
  import { Eye, EyeOff } from "lucide-svelte";

  // A password field with a button to show what was typed.
  export let value = "";
  export let placeholder = "";
  export let invalid = false;
  export let onInput: (value: string) => void = () => {};

  let visible = false;
</script>

<span class="relative block">
  <input
    type={visible ? "text" : "password"}
    autocomplete="new-password"
    spellcheck="false"
    {placeholder}
    {value}
    on:input={(e) => onInput(e.currentTarget.value)}
    class="w-full bg-surface-2 border rounded pl-3 pr-9 py-1.5 text-[0.88rem] text-text-bright placeholder:text-text-muted outline-none focus-visible:ring-2 focus-visible:ring-primary/60
      {invalid ? 'border-danger' : 'border-border'}"
  />
  <button
    type="button"
    class="absolute right-1.5 top-1/2 -translate-y-1/2 grid place-items-center w-7 h-7 rounded text-text-muted hover:text-text-bright"
    title={$_(visible ? "user.hide" : "user.show")}
    aria-label={$_(visible ? "user.hide" : "user.show")}
    on:click={() => (visible = !visible)}
  >
    {#if visible}<EyeOff size={15} />{:else}<Eye size={15} />{/if}
  </button>
</span>

<script lang="ts">
  import { altHeld } from "../lib/mnemonics";

  // A label with its mnemonic letter, underlined while Alt is held. All of
  // it inside one span: the buttons are flex boxes with a gap, and loose
  // pieces of text would each get that gap ("A trás").
  export let text: string;
  export let index = -1;
</script>

<span>{#if index < 0}{text}{:else}{text.slice(0, index)}<span class:mnemonic={$altHeld}>{text[index]}</span>{text.slice(index + 1)}{/if}</span>

<style>
  .mnemonic {
    text-decoration: underline;
    text-decoration-thickness: 2px;
    text-underline-offset: 3px;
  }
</style>

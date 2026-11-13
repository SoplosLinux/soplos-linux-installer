<script lang="ts">
  import { _ } from "svelte-i18n";
  import type { ComponentType } from "svelte";
  import { ArrowLeft, ArrowRight, Download, RotateCcw, LoaderCircle, CircleCheck, CircleX, ScrollText, Presentation, X } from "lucide-svelte";
  import { installStep, installPercent, installDone, showLog } from "../lib/install";
  import { assignMnemonics, keyOf, mnemonicActions } from "../lib/mnemonics";
  import Mnemonic from "./Mnemonic.svelte";

  // The buttons under every step. Back and Next move between the steps that
  // collect the plan; on the summary Next becomes Install; while installing
  // it shows the step running, "Show details" and, at the end, the way back
  // (dry run) or Close (failure); on the last step only Restart remains.
  // Every button has its Alt mnemonic (lib/mnemonics.ts).
  export let step: string;
  export let isFirst = false;
  export let canContinue = true;
  export let onBack: () => void;
  export let onNext: () => void;
  export let onInstall: () => void;
  export let onRestart: () => void;
  /** Back to the summary after a dry run. */
  export let onReturn: () => void = () => {};
  /** Closes the installer after a failed installation. */
  export let onClose: () => void = () => {};

  interface Button {
    id: string;
    label: string;
    icon: ComponentType;
    /** Icon after the label (Next). */
    trailing?: boolean;
    primary: boolean;
    disabled: boolean;
    action: () => void;
  }

  $: buttons = ((): Button[] => {
    if (step === "install") {
      const list: Button[] = [
        {
          id: "details",
          label: $_($showLog ? "install.hideDetails" : "install.showDetails"),
          icon: $showLog ? Presentation : ScrollText,
          primary: false,
          disabled: false,
          action: () => showLog.update((v) => !v),
        },
      ];
      if ($installDone?.dryRun)
        list.push({ id: "return", label: $_("install.back"), icon: ArrowLeft, primary: true, disabled: false, action: onReturn });
      else if ($installDone && !$installDone.success)
        list.push({ id: "close", label: $_("install.close"), icon: X, primary: true, disabled: false, action: onClose });
      return list;
    }
    if (step === "finished") {
      return [{ id: "restart", label: $_("nav.restart"), icon: RotateCcw, primary: true, disabled: false, action: onRestart }];
    }
    const back: Button = { id: "back", label: $_("nav.back"), icon: ArrowLeft, primary: false, disabled: isFirst, action: onBack };
    const forward: Button =
      step === "summary"
        ? { id: "install", label: $_("nav.install"), icon: Download, primary: true, disabled: !canContinue, action: onInstall }
        : { id: "next", label: $_("nav.next"), icon: ArrowRight, trailing: true, primary: true, disabled: !canContinue, action: onNext };
    return [back, forward];
  })();

  $: marks = assignMnemonics(buttons.map((b) => b.label));

  // The letters of the buttons on screen, for App.svelte's key handler. A
  // disabled button does nothing.
  $: mnemonicActions.set(
    Object.fromEntries(
      buttons
        .map((b, i) => [marks[i] >= 0 ? keyOf(b.label[marks[i]]) : "", () => !b.disabled && b.action()] as const)
        .filter(([key]) => key !== ""),
    ),
  );

  // Back on the left and the main button on the right; while installing,
  // the status on the left and the buttons together on the right.
  $: left = step === "install" || step === "finished" ? [] : buttons.slice(0, 1);
  $: right = step === "install" || step === "finished" ? buttons : buttons.slice(1);
  const indexOf = (b: Button) => buttons.indexOf(b);
</script>

<div class="shrink-0 flex items-center justify-between gap-3 px-7 py-3.5 border-t border-border bg-panel">
  {#if step === "install"}
    <p class="flex-1 min-w-0 flex items-center gap-2 text-[0.88rem] {$installDone ? ($installDone.success ? 'text-success' : 'text-danger') : 'text-text-bright'}">
      {#if !$installDone}
        <LoaderCircle size={16} class="shrink-0 animate-spin text-primary" />
        <span class="truncate">{$installStep ? $_(`install.steps.${$installStep}`) : $_("install.starting")}</span>
        {#if $installPercent !== null}<span class="text-text-muted tabular-nums">{$installPercent}%</span>{/if}
      {:else if $installDone.success}
        <CircleCheck size={16} class="shrink-0" />
        <span class="truncate">{$_("install.dryRunDone")}</span>
      {:else}
        <CircleX size={16} class="shrink-0" />
        <span class="truncate" title={$installDone.message}>{$_("install.failed", { values: { message: $installDone.message } })}</span>
      {/if}
    </p>
  {:else}
    <div>
      {#each left as b (b.id)}
        <button
          class="flex items-center gap-2 px-4 py-2 rounded border border-border text-text hover:bg-surface-2 text-[0.88rem] disabled:opacity-40 disabled:cursor-not-allowed"
          disabled={b.disabled}
          on:click={b.action}
        >
          <svelte:component this={b.icon} size={16} />
          <Mnemonic text={b.label} index={marks[indexOf(b)]} />
        </button>
      {/each}
    </div>
  {/if}

  <div class="flex items-center gap-2 shrink-0">
    {#each right as b (b.id)}
      <button
        class="flex items-center gap-2 rounded text-[0.88rem] disabled:opacity-40 disabled:cursor-not-allowed
          {b.primary ? 'px-5 py-2 bg-primary hover:bg-primary-hover text-text-bright font-semibold' : 'px-4 py-2 border border-border text-text hover:bg-surface-2'}"
        disabled={b.disabled}
        on:click={b.action}
      >
        {#if !b.trailing}<svelte:component this={b.icon} size={16} />{/if}
        <Mnemonic text={b.label} index={marks[indexOf(b)]} />
        {#if b.trailing}<svelte:component this={b.icon} size={16} />{/if}
      </button>
    {/each}
  </div>
</div>

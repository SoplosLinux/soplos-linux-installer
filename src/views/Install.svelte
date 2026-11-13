<script lang="ts">
  import { onMount, onDestroy, tick } from "svelte";
  import { listen, type UnlistenFn } from "@tauri-apps/api/event";
  import Presentation from "../presentation/Presentation.svelte";
  import { api } from "../lib/api";
  import { plan, current, reached, STEPS } from "../lib/wizard";
  import { installStep, installPercent, installDone, showLog, resetInstall } from "../lib/install";

  // While the helper works (DESIGN.md, section 9): the presentation fills the
  // space. The bottom bar (WizardNav) shows the step running and the buttons,
  // and the thin progress bar runs under it, as in the other Soplos apps.
  // "Show details" swaps the presentation for the log; a failure shows the
  // log by itself, since then it is what matters.

  type HelperEvent =
    | { kind: "step"; id: string; index: number; total: number }
    | { kind: "progress"; percent: number }
    | { kind: "log"; line: string }
    | { kind: "done"; success: boolean; message: string; dryRun: boolean };

  let lines: string[] = [];
  let logBox: HTMLDivElement;
  let edition = "";
  let unlisten: UnlistenFn | undefined;

  async function scrollLog() {
    await tick();
    logBox?.scrollTo({ top: logBox.scrollHeight });
  }

  function onEvent(e: HelperEvent) {
    if (e.kind === "step") installStep.set(e.id);
    else if (e.kind === "progress") installPercent.set(e.percent);
    else if (e.kind === "log") {
      lines = [...lines, e.line];
      scrollLog();
    } else if (e.kind === "done") {
      installDone.set({ success: e.success, message: e.message, dryRun: e.dryRun });
      if (!e.success) showLog.set(true);
      // A real installation that ends well goes to the last step.
      else if (!e.dryRun) {
        reached.set(STEPS.indexOf("finished"));
        current.set(STEPS.indexOf("finished"));
      }
    }
  }

  $: if ($showLog) scrollLog();

  onMount(async () => {
    resetInstall();
    api
      .invoke<{ edition: string }>("get_environment_info")
      .then((env) => (edition = env.edition))
      .catch(() => {});
    unlisten = await listen<HelperEvent>("install-event", (event) => onEvent(event.payload));
    try {
      await api.invoke("start_install", { plan: $plan });
    } catch (error) {
      installDone.set({ success: false, message: String(error), dryRun: false });
      showLog.set(true);
    }
  });
  onDestroy(() => unlisten?.());
</script>

<div class="h-full min-h-0 flex flex-col gap-4">
  <!-- Both stay mounted: switching never restarts the presentation nor
       loses the log. -->
  <div class="flex-1 min-h-0 {$showLog ? 'hidden' : ''}">
    <Presentation {edition} bootloader={$plan.bootloader} />
  </div>
  <div
    bind:this={logBox}
    class="{$showLog ? '' : 'hidden'} flex-1 min-h-0 overflow-y-auto rounded-lg border border-border bg-panel p-3 font-mono text-[0.75rem] leading-relaxed text-text whitespace-pre-wrap break-all"
  >
    {#each lines as line}
      <div class={line.startsWith("$") ? "text-text-bright" : line.startsWith("ERROR") ? "text-danger" : ""}>{line}</div>
    {/each}
  </div>
</div>

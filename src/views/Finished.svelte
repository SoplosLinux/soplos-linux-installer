<script lang="ts">
  import { onMount } from "svelte";
  import { _ } from "svelte-i18n";
  import { CircleCheck, Usb, Sparkles } from "lucide-svelte";
  import { api } from "../lib/api";

  // The end: the system is installed. Restart is in the bottom bar (Alt and
  // its letter, as every button).

  let name = "Soplos Linux";
  onMount(async () => {
    const req = await api.invoke<{ editionName: string }>("check_requirements").catch(() => null);
    if (req?.editionName) name = req.editionName;
  });
</script>

<div class="flex flex-col gap-4">
  <div class="flex items-center gap-4 rounded-lg border border-success/40 bg-success/10 p-5">
    <CircleCheck size={36} class="text-success shrink-0" />
    <div class="min-w-0">
      <p class="text-[1rem] text-text-bright font-semibold">{$_("finished.done", { values: { name } })}</p>
      <p class="text-[0.85rem] text-text mt-1">{$_("finished.restart")}</p>
    </div>
  </div>
  <div class="flex items-center gap-3 rounded-lg border border-border bg-surface p-4">
    <Usb size={20} class="text-primary shrink-0" />
    <p class="text-[0.85rem] text-text">{$_("finished.media")}</p>
  </div>
  <div class="flex items-center gap-3 rounded-lg border border-border bg-surface p-4">
    <Sparkles size={20} class="text-primary shrink-0" />
    <p class="text-[0.85rem] text-text">{$_("finished.welcome")}</p>
  </div>
</div>

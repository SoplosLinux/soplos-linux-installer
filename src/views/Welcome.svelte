<script lang="ts">
  import { onMount, onDestroy } from "svelte";
  import { _, locale } from "svelte-i18n";
  import { Globe, ChevronDown, HardDrive, MemoryStick, Wifi, BatteryCharging, CircleCheck, CircleX, TriangleAlert, LoaderCircle, RefreshCw } from "lucide-svelte";
  import { SUPPORTED_LOCALES, LOCALE_NAMES, setLocale, type SupportedLocale } from "../i18n";
  import { api } from "../lib/api";
  import { canContinue } from "../lib/wizard";

  // The language picked here is the one of the installer from now on.
  function onLanguageChange(e: Event) {
    setLocale((e.currentTarget as HTMLSelectElement).value as SupportedLocale);
  }

  // DESIGN.md, section 3: disk, memory and internet stop the installation,
  // the battery only warns. The figures come from the data file of the edition.
  interface Requirements {
    editionName: string;
    diskGb: number | null;
    diskRequiredGb: number | null;
    memoryGb: number | null;
    memoryRequiredGb: number | null;
    onBattery: boolean | null;
  }

  let req: Requirements | null = null;
  let internet: boolean | null = null;
  let checking = false;

  async function checkAll() {
    checking = true;
    internet = null;
    const [r, net] = await Promise.all([
      api.invoke<Requirements>("check_requirements").catch(() => null),
      api.invoke<boolean>("check_internet").catch(() => false),
    ]);
    req = r;
    internet = net;
    checking = false;
  }

  // The cable or the charger can be plugged in while this screen is open: the
  // two checks that change by themselves are repeated every few seconds.
  let timer: ReturnType<typeof setInterval>;
  let polling = false;
  async function poll() {
    if (checking || polling) return;
    polling = true;
    const [r, net] = await Promise.all([
      api.invoke<Requirements>("check_requirements").catch(() => req),
      internet ? Promise.resolve(true) : api.invoke<boolean>("check_internet").catch(() => false),
    ]);
    if (!checking) {
      req = r;
      internet = net;
    }
    polling = false;
  }

  onMount(() => {
    checkAll();
    timer = setInterval(poll, 5000);
  });
  onDestroy(() => clearInterval(timer));

  const meets = (have: number | null | undefined, need: number | null | undefined) =>
    have != null && (need == null || have >= need);

  $: diskOk = req ? meets(req.diskGb, req.diskRequiredGb) : null;
  $: memoryOk = req ? meets(req.memoryGb, req.memoryRequiredGb) : null;
  $: ready = diskOk === true && memoryOk === true && internet === true;
  $: canContinue.set(ready);

  $: fmt = new Intl.NumberFormat($locale ?? "en", { maximumFractionDigits: 1 });
  const gb = (value: number | null | undefined) => (value == null ? "" : fmt.format(value));

  type Status = "ok" | "fail" | "warn" | "pending";
  interface Row {
    icon: typeof HardDrive;
    label: string;
    detail: string;
    status: Status;
  }

  let rows: Row[] = [];
  $: {
    fmt;
    $locale;
    const list: Row[] = [];
    if (req) {
      list.push({
        icon: HardDrive,
        label: $_("welcome.checks.disk"),
        detail:
          req.diskGb == null
            ? $_("welcome.checks.diskNone")
            : $_(req.diskRequiredGb == null ? "welcome.checks.diskFound" : "welcome.checks.diskDetail", {
                values: { size: gb(req.diskGb), required: gb(req.diskRequiredGb) },
              }),
        status: diskOk ? "ok" : "fail",
      });
      list.push({
        icon: MemoryStick,
        label: $_("welcome.checks.memory"),
        detail: $_(req.memoryRequiredGb == null ? "welcome.checks.memoryFound" : "welcome.checks.memoryDetail", {
          values: { size: gb(req.memoryGb), required: gb(req.memoryRequiredGb) },
        }),
        status: memoryOk ? "ok" : "fail",
      });
    }
    list.push({
      icon: Wifi,
      label: $_("welcome.checks.internet"),
      detail:
        internet === null ? $_("welcome.checks.checking") : $_(internet ? "welcome.checks.internetOk" : "welcome.checks.internetFail"),
      status: internet === null ? "pending" : internet ? "ok" : "fail",
    });
    // Without a battery there is nothing to say about the power.
    if (req && req.onBattery != null) {
      list.push({
        icon: BatteryCharging,
        label: $_("welcome.checks.power"),
        detail: $_(req.onBattery ? "welcome.checks.powerBattery" : "welcome.checks.powerAc"),
        status: req.onBattery ? "warn" : "ok",
      });
    }
    rows = list;
  }

  const statusIcon = { ok: CircleCheck, fail: CircleX, warn: TriangleAlert, pending: LoaderCircle };
  const statusColor = { ok: "text-success", fail: "text-danger", warn: "text-warning", pending: "text-text-muted animate-spin" };
</script>

<div class="flex flex-col gap-5">
  <div class="flex items-center gap-4 rounded-lg border border-border bg-surface p-5">
    <img src="/assets/icons/sli.png" alt="Soplos Linux Installer" class="w-16 h-16 object-contain shrink-0" />
    <div class="min-w-0 flex-1">
      <p class="text-[0.95rem] text-text-bright font-semibold">{$_("welcome.greeting")}</p>
      <p class="text-[0.85rem] text-text mt-1">{$_("welcome.intro")}</p>
    </div>
  </div>

  <label class="flex items-center gap-3 rounded-lg border border-border bg-surface p-4">
    <Globe size={18} class="text-primary shrink-0" />
    <span class="text-[0.88rem] text-text-bright flex-1">{$_("welcome.language")}</span>
    <!-- appearance-none: WebKitGTK otherwise draws the native widget of the
         system, which ignores the colors of the theme. -->
    <span class="relative">
      <select
        class="appearance-none bg-surface-2 border border-border rounded pl-3 pr-8 py-1.5 text-[0.88rem] text-text-bright outline-none focus-visible:ring-2 focus-visible:ring-primary/60"
        value={$locale}
        on:change={onLanguageChange}
      >
        {#each SUPPORTED_LOCALES as code}
          <option value={code}>{LOCALE_NAMES[code]}</option>
        {/each}
      </select>
      <ChevronDown size={14} class="pointer-events-none absolute right-2.5 top-1/2 -translate-y-1/2 text-text-muted" />
    </span>
  </label>

  <div class="rounded-lg border border-border bg-surface">
    <div class="flex items-center justify-between gap-3 px-4 pt-4 pb-2">
      <p class="text-[0.8rem] font-semibold uppercase tracking-wide text-text-muted">
        {req?.editionName ? $_("welcome.checks.titleFor", { values: { edition: req.editionName } }) : $_("welcome.checks.title")}
      </p>
      <button
        type="button"
        class="flex items-center gap-1.5 rounded px-2.5 py-1 text-[0.8rem] text-text hover:bg-surface-2 hover:text-text-bright disabled:opacity-50 outline-none focus-visible:ring-2 focus-visible:ring-primary/60"
        disabled={checking}
        on:click={checkAll}
      >
        <RefreshCw size={14} class={checking ? "animate-spin" : ""} />
        {$_("welcome.checks.retry")}
      </button>
    </div>

    <ul class="divide-y divide-border">
      {#each rows as row (row.label)}
        <li class="flex items-center gap-3 px-4 py-3">
          <svelte:component this={row.icon} size={18} class="text-primary shrink-0" />
          <div class="min-w-0 flex-1">
            <p class="text-[0.88rem] text-text-bright">{row.label}</p>
            <p class="text-[0.8rem] text-text mt-0.5">{row.detail}</p>
          </div>
          <svelte:component this={statusIcon[row.status]} size={20} class="shrink-0 {statusColor[row.status]}" />
        </li>
      {/each}
    </ul>
  </div>
</div>

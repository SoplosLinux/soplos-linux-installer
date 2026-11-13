<script context="module" lang="ts">
  // Once the user touches language or keyboard, picking another place no
  // longer overwrites them. Kept here so it survives going back and forth.
  let languageTouched = false;
  let keyboardTouched = false;
</script>

<script lang="ts">
  import { onMount, onDestroy } from "svelte";
  import { _, locale } from "svelte-i18n";
  import { MapPin, Languages, Keyboard, LoaderCircle, Clock } from "lucide-svelte";
  import { api } from "../lib/api";
  import { plan, canContinue } from "../lib/wizard";
  import {
    type Zone,
    type Layout,
    cityOf,
    countryName,
    localeName,
    bcp47,
    FALLBACK_ZONE,
    guessLanguage,
    guessFormats,
    guessKeyboard,
  } from "../lib/location";
  import TimezoneMap from "../components/TimezoneMap.svelte";
  import Select from "../components/Select.svelte";

  let zones: Zone[] = [];
  let locales: string[] = [];
  let layouts: Layout[] = [];
  let detecting = false;

  $: ui = $locale?.split("-")[0] ?? "en";
  $: canContinue.set(!!($plan.timezone && $plan.locale && $plan.formats && $plan.keyboard.layout));

  // The descriptions of the keyboards follow the language of the installer.
  let layoutsFor = "";
  $: if (ui !== layoutsFor) loadLayouts(ui);
  async function loadLayouts(language: string) {
    layoutsFor = language;
    const list = await api.invoke<Layout[]>("get_keyboard_layouts", { language }).catch(() => [] as Layout[]);
    if (language === layoutsFor) layouts = list;
  }

  function zone(id: string) {
    return zones.find((z) => z.id === id);
  }

  /** Language, formats and keyboard follow the place until the user changes them. */
  function applyPlace(id: string) {
    const country = zone(id)?.country ?? "";
    const language = guessLanguage(ui, country, locales);
    plan.update((p) => ({
      ...p,
      timezone: id,
      locale: language,
      formats: guessFormats(ui, country, locales, language),
    }));
    if (layouts.length) setKeyboard(guessKeyboard(country, ui, layouts), "");
    else pendingKeyboardFor = country;
  }
  let pendingKeyboardFor: string | null = null;
  $: if (pendingKeyboardFor !== null && layouts.length) {
    const country = pendingKeyboardFor;
    pendingKeyboardFor = null;
    setKeyboard(guessKeyboard(country, ui, layouts), "");
  }

  function selectZone(id: string) {
    userPicked = true;
    if (languageTouched || keyboardTouched) {
      const country = zone(id)?.country ?? "";
      plan.update((p) => ({ ...p, timezone: id }));
      if (!languageTouched) {
        const language = guessLanguage(ui, country, locales);
        plan.update((p) => ({ ...p, locale: language, formats: guessFormats(ui, country, locales, language) }));
      }
      if (!keyboardTouched && layouts.length) setKeyboard(guessKeyboard(country, ui, layouts), "");
    } else {
      applyPlace(id);
    }
  }

  function setKeyboard(layout: string, variant: string) {
    plan.update((p) => ({ ...p, keyboard: { layout, variant } }));
    // The test field types with the layout chosen: it is put on the live
    // session. If the desktop does not allow it, the field still works.
    api.invoke("set_live_keyboard", { layout, variant }).catch(() => {});
  }

  let userPicked = false;

  onMount(async () => {
    const data = await api
      .invoke<{ zones: Zone[]; locales: string[] }>("get_location_data")
      .catch(() => ({ zones: [] as Zone[], locales: [] as string[] }));
    zones = data.zones;
    locales = data.locales.filter((l) => /^[a-z]{2,3}_[A-Z]{2}\./.test(l));
    if ($plan.timezone) return; // Coming back to the step keeps what was chosen.

    const fallback = FALLBACK_ZONE[ui] ?? "Europe/London";
    applyPlace(zone(fallback) ? fallback : (zones[0]?.id ?? ""));
    detecting = true;
    const found = await api.invoke<string | null>("detect_timezone").catch(() => null);
    detecting = false;
    if (found && zone(found) && !userPicked && !languageTouched && !keyboardTouched) applyPlace(found);
  });

  // The local time of the zone chosen, as a check that it is the right one.
  let now = new Date();
  const clock = setInterval(() => (now = new Date()), 15000);
  onDestroy(() => clearInterval(clock));
  $: localTime = (() => {
    try {
      return $plan.timezone
        ? new Intl.DateTimeFormat($locale ?? "en", { timeStyle: "short", timeZone: $plan.timezone }).format(now)
        : "";
    } catch {
      return "";
    }
  })();

  $: nameOf = (z: Zone) => `${cityOf(z.id)} · ${countryName(z.country, ui)}`;

  $: regions = [...new Set(zones.map((z) => z.id.split("/")[0]))]
    .map((r) => ({ value: r, label: $_(`location.regions.${r}`, { default: r }) }))
    .sort((a, b) => a.label.localeCompare(b.label, ui));
  $: region = $plan.timezone.split("/")[0];
  $: cities = zones
    .filter((z) => z.id.split("/")[0] === region)
    .map((z) => ({ value: z.id, label: nameOf(z) }))
    .sort((a, b) => a.label.localeCompare(b.label, ui));

  function selectRegion(r: string) {
    const first = zones
      .filter((z) => z.id.split("/")[0] === r)
      .sort((a, b) => cityOf(a.id).localeCompare(cityOf(b.id), ui))[0];
    if (first) selectZone(first.id);
  }

  $: localeOptions = locales
    .map((l) => ({ value: l, label: localeName(l, ui) }))
    .sort((a, b) => a.label.localeCompare(b.label, ui));

  $: formatsExample = (() => {
    if (!$plan.formats) return "";
    try {
      const tag = bcp47($plan.formats);
      const date = new Intl.DateTimeFormat(tag, { dateStyle: "long" }).format(now);
      const number = new Intl.NumberFormat(tag, { minimumFractionDigits: 2 }).format(1234567.89);
      return `${date} · ${number}`;
    } catch {
      return "";
    }
  })();

  $: layoutOptions = layouts.map((l) => ({ value: l.name, label: l.description }));
  $: variantOptions = [
    { value: "", label: $_("location.variantDefault") },
    ...(layouts.find((l) => l.name === $plan.keyboard.layout)?.variants ?? []).map((v) => ({
      value: v.name,
      label: v.description,
    })),
  ];
</script>

<div class="flex flex-col gap-4">
  <div class="rounded-lg border border-border bg-surface overflow-hidden">
    {#if zones.length}
      <TimezoneMap {zones} selected={$plan.timezone} onSelect={selectZone} {nameOf} zoom={userPicked} />
    {:else}
      <div class="aspect-[1000/364] grid place-items-center text-text-muted">
        <LoaderCircle size={22} class="animate-spin" />
      </div>
    {/if}
    <div class="flex flex-wrap items-center gap-3 px-4 py-3 border-t border-border">
      <MapPin size={18} class="text-primary shrink-0" />
      <div class="w-40"><Select label={$_("location.region")} value={region} options={regions} onChange={selectRegion} /></div>
      <div class="flex-1 min-w-[12rem]">
        <Select label={$_("location.zone")} value={$plan.timezone} options={cities} onChange={selectZone} />
      </div>
      <span class="flex items-center gap-1.5 text-[0.82rem] text-text whitespace-nowrap">
        {#if detecting}
          <LoaderCircle size={14} class="animate-spin text-text-muted" />
          {$_("location.detecting")}
        {:else if localTime}
          <Clock size={14} class="text-text-muted" />
          {$_("location.localTime", { values: { time: localTime } })}
        {/if}
      </span>
    </div>
  </div>

  <div class="grid grid-cols-2 gap-4">
    <div class="rounded-lg border border-border bg-surface p-4 flex flex-col gap-2.5 min-w-0">
      <p class="flex items-center gap-2 text-[0.88rem] text-text-bright font-semibold">
        <Languages size={18} class="text-primary" />
        {$_("location.languageTitle")}
      </p>
      <div class="grid grid-cols-[6.5rem_minmax(0,1fr)] items-center gap-2">
        <span class="text-[0.82rem] text-text">{$_("location.language")}</span>
        <Select
          label={$_("location.language")}
          value={$plan.locale}
          options={localeOptions}
          onChange={(v) => {
            languageTouched = true;
            plan.update((p) => ({ ...p, locale: v }));
          }}
        />
      </div>
      <div class="grid grid-cols-[6.5rem_minmax(0,1fr)] items-center gap-2">
        <span class="text-[0.82rem] text-text">{$_("location.formats")}</span>
        <Select
          label={$_("location.formats")}
          value={$plan.formats}
          options={localeOptions}
          onChange={(v) => {
            languageTouched = true;
            plan.update((p) => ({ ...p, formats: v }));
          }}
        />
      </div>
      <p class="text-[0.78rem] text-text-muted truncate pl-[7rem]" title={formatsExample}>
        {formatsExample}
      </p>
    </div>

    <div class="rounded-lg border border-border bg-surface p-4 flex flex-col gap-2.5 min-w-0">
      <p class="flex items-center gap-2 text-[0.88rem] text-text-bright font-semibold">
        <Keyboard size={18} class="text-primary" />
        {$_("location.keyboard")}
      </p>
      <div class="grid grid-cols-[6.5rem_minmax(0,1fr)] items-center gap-2">
        <span class="text-[0.82rem] text-text">{$_("location.layout")}</span>
        <Select
          label={$_("location.layout")}
          value={$plan.keyboard.layout}
          options={layoutOptions}
          onChange={(v) => {
            keyboardTouched = true;
            setKeyboard(v, "");
          }}
        />
      </div>
      <div class="grid grid-cols-[6.5rem_minmax(0,1fr)] items-center gap-2">
        <span class="text-[0.82rem] text-text">{$_("location.variant")}</span>
        <Select
          label={$_("location.variant")}
          value={$plan.keyboard.variant}
          options={variantOptions}
          onChange={(v) => {
            keyboardTouched = true;
            setKeyboard($plan.keyboard.layout, v);
          }}
        />
      </div>
      <input
        type="text"
        spellcheck="false"
        placeholder={$_("location.test")}
        class="bg-surface-2 border border-border rounded px-3 py-1.5 text-[0.88rem] text-text-bright placeholder:text-text-muted outline-none focus-visible:ring-2 focus-visible:ring-primary/60"
      />
    </div>
  </div>
</div>

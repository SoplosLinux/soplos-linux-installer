<script lang="ts">
  import { geoEquirectangular, geoPath } from "d3-geo";
  import { feature } from "topojson-client";
  import type { Topology, GeometryCollection } from "topojson-specification";
  import type { FeatureCollection } from "geojson";
  import world from "world-atlas/countries-50m.json";
  import { COUNTRY_NUMERIC } from "../lib/countryNumeric";
  import type { Zone } from "../lib/location";
  import { tweened } from "svelte/motion";
  import { cubicInOut } from "svelte/easing";
  import { _ } from "svelte-i18n";
  import { ZoomOut } from "lucide-svelte";

  // The world with one dot per time zone of tzdata. A click picks the zone
  // nearest to it; the country of the chosen zone is lit up. Everything is
  // drawn from data built into the program, so the map needs no connection.
  export let zones: Zone[] = [];
  export let selected = "";
  export let onSelect: (id: string) => void = () => {};
  /** The name shown over a dot under the pointer. */
  export let nameOf: (zone: Zone) => string = (zone) => zone.id;
  /** Zoom in on the chosen zone. On when the user picks it, off while the
   * place is only guessed, so the whole world shows first. */
  export let zoom = false;

  // Antarctica and the far north are left out: no zone the user would pick
  // lives there and they would take a third of the height.
  const NORTH = 75;
  const SOUTH = -56;
  const W = 1000;
  const H = Math.round((W * (NORTH - SOUTH)) / 360);

  const projection = geoEquirectangular().fitSize([W, H], {
    type: "MultiPoint",
    coordinates: [
      [-180, NORTH],
      [180, SOUTH],
    ],
  });
  const path = geoPath(projection);

  const topology = world as unknown as Topology<{ countries: GeometryCollection }>;
  const countries = (feature(topology, topology.objects.countries) as FeatureCollection).features.map((f) => ({
    id: String(f.id ?? ""),
    d: path(f) ?? "",
  }));

  $: points = zones.map((zone) => {
    const [x, y] = projection([zone.lon, zone.lat]) ?? [0, 0];
    return { zone, x, y };
  });
  $: current = points.find((p) => p.zone.id === selected);
  $: litCountry = current ? COUNTRY_NUMERIC[current.zone.country] : "";

  // --- Zoom: the viewBox glides to a region around the chosen zone. -------

  const FULL = { x: 0, y: 0, w: W, h: H };
  const ZOOM = 4;
  const view = tweened(FULL, { duration: 650, easing: cubicInOut });

  function regionAround(x: number, y: number) {
    const w = W / ZOOM;
    const h = H / ZOOM;
    return {
      x: Math.min(Math.max(x - w / 2, 0), W - w),
      y: Math.min(Math.max(y - h / 2, 0), H - h),
      w,
      h,
    };
  }

  // Only a new choice zooms in: after "see the whole map" it stays out
  // until another place is picked.
  let focused = "";
  $: if (zoom && current && current.zone.id !== focused) {
    focused = current.zone.id;
    view.set(regionAround(current.x, current.y));
  }
  $: if (!zoom && focused) {
    focused = "";
    view.set(FULL);
  }
  $: zoomedIn = $view.w < W - 1;

  /** Dots, lines and markers keep their size on screen at any zoom. */
  $: k = $view.w / W;

  let svg: SVGSVGElement;
  let hover: (typeof points)[number] | null = null;

  /** The point of the map under the pointer, in the units of the viewBox. */
  function toMap(e: MouseEvent) {
    const ctm = svg.getScreenCTM();
    if (!ctm) return null;
    const p = new DOMPoint(e.clientX, e.clientY).matrixTransform(ctm.inverse());
    return { x: p.x, y: p.y };
  }

  function nearest(x: number, y: number, limit = Infinity) {
    let best: (typeof points)[number] | null = null;
    let bestDistance = limit * limit;
    for (const p of points) {
      const d = (p.x - x) ** 2 + (p.y - y) ** 2;
      if (d < bestDistance) {
        best = p;
        bestDistance = d;
      }
    }
    return best;
  }

  function click(e: MouseEvent) {
    const p = toMap(e);
    const found = p && nearest(p.x, p.y);
    if (!found) return;
    onSelect(found.zone.id);
    // A click always zooms, also on the zone that was already chosen (the
    // detected one), which changes nothing else and so would not zoom.
    focused = found.zone.id;
    view.set(regionAround(found.x, found.y));
  }

  function move(e: MouseEvent) {
    const p = toMap(e);
    hover = p ? nearest(p.x, p.y, 18 * k) : null;
  }
</script>

<div class="relative">
  <!-- svelte-ignore a11y-click-events-have-key-events a11y-no-noninteractive-element-interactions -->
  <svg
    bind:this={svg}
    viewBox="{$view.x} {$view.y} {$view.w} {$view.h}"
    class="block w-full h-auto cursor-crosshair select-none"
    role="img"
    on:click={click}
    on:mousemove={move}
    on:mouseleave={() => (hover = null)}
  >
    <rect x={-W} y={-H} width={W * 3} height={H * 3} class="fill-panel" />
    {#each countries as country}
      <path
        d={country.d}
        class="{country.id && country.id === litCountry ? 'fill-primary/40' : 'fill-surface-2'} stroke-border"
        stroke-width={0.6 * k}
      />
    {/each}
    {#each points as p (p.zone.id)}
      <circle cx={p.x} cy={p.y} r={2 * Math.max(k, 0.5)} class="fill-text-muted" opacity="0.55" />
    {/each}
    {#if hover && hover !== current}
      <circle cx={hover.x} cy={hover.y} r={4 * k} class="fill-secondary" />
    {/if}
    {#if current}
      <circle cx={current.x} cy={current.y} r={6 * k} class="fill-primary" opacity="0.5">
        <animate attributeName="r" from={6 * k} to={20 * k} dur="1.6s" repeatCount="indefinite" />
        <animate attributeName="opacity" from="0.5" to="0" dur="1.6s" repeatCount="indefinite" />
      </circle>
      <circle cx={current.x} cy={current.y} r={6 * k} class="fill-primary stroke-white" stroke-width={2 * k} />
    {/if}
  </svg>

  {#if hover}
    <div
      class="pointer-events-none absolute -translate-x-1/2 -translate-y-full -mt-2 whitespace-nowrap rounded bg-panel/95 border border-border px-2 py-0.5 text-[0.75rem] text-text-bright shadow"
      style="left: {((hover.x - $view.x) / $view.w) * 100}%; top: {((hover.y - $view.y) / $view.h) * 100}%;"
    >
      {nameOf(hover.zone)}
    </div>
  {/if}

  {#if zoomedIn}
    <button
      type="button"
      class="absolute top-2.5 right-2.5 flex items-center gap-1.5 rounded bg-panel/90 border border-border px-2.5 py-1 text-[0.78rem] text-text-bright hover:border-primary/60 hover:text-primary shadow"
      on:click={() => view.set(FULL)}
    >
      <ZoomOut size={14} />
      {$_("location.zoomOut")}
    </button>
  {/if}
</div>

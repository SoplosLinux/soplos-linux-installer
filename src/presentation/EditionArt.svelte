<script lang="ts">
  // A drawing of the desktop of an edition: a screen with its wallpaper, its
  // panels where that desktop has them and an open window. Vector, so it is
  // sharp at any size, and drawn here, so it needs no screenshots.
  //   Tyron (Xfce): a thin top panel and a dock at the bottom.
  //   Tyson (Plasma): one bottom panel with launcher, tasks and tray.
  //   Boro (GNOME): a top bar with the clock in the middle and a dash.

  export let edition: "tyron" | "tyson" | "boro" | string = "tyron";
  /** Animate the parts in: off for the small cards. */
  export let animated = true;

  const uid = Math.random().toString(36).slice(2, 8);

  const WALL: Record<string, [string, string, string]> = {
    tyron: ["#2a1606", "#ff7d2e", "#fec431"],
    tyson: ["#1d0f08", "#ef5a33", "#ffb52e"],
    boro: ["#24130a", "#ff9a1f", "#ffd23f"],
  };
  $: [w0, w1, w2] = WALL[edition] ?? WALL.tyron;
</script>

<svg viewBox="0 0 320 210" class:animated role="img" aria-label={edition}>
  <defs>
    <linearGradient id="wall-{uid}" x1="0" y1="0" x2="1" y2="1">
      <stop offset="0" stop-color={w0} />
      <stop offset="0.65" stop-color={w1} stop-opacity="0.55" />
      <stop offset="1" stop-color={w2} stop-opacity="0.8" />
    </linearGradient>
    <radialGradient id="glow-{uid}" cx="0.72" cy="0.38" r="0.6">
      <stop offset="0" stop-color={w2} stop-opacity="0.55" />
      <stop offset="1" stop-color={w2} stop-opacity="0" />
    </radialGradient>
    <clipPath id="screen-{uid}"><rect x="10" y="8" width="300" height="180" rx="8" /></clipPath>
  </defs>

  <!-- the monitor -->
  <rect x="4" y="2" width="312" height="192" rx="12" fill="#121214" stroke="rgba(255,255,255,0.12)" />
  <rect x="140" y="194" width="40" height="8" fill="#1a1a1d" />
  <rect x="116" y="202" width="88" height="5" rx="2.5" fill="#1a1a1d" />

  <g clip-path="url(#screen-{uid})">
    <rect x="10" y="8" width="300" height="180" fill="url(#wall-{uid})" />
    <rect x="10" y="8" width="300" height="180" fill="url(#glow-{uid})" />
    <!-- the Soplos waves of the wallpaper -->
    <path d="M10 150 C 80 120, 140 175, 210 140 S 300 120, 310 132 L310 188 L10 188 Z" fill="rgba(0,0,0,0.25)" class="part p1" />
    <path d="M10 165 C 90 140, 150 190, 230 158 S 300 150, 310 156 L310 188 L10 188 Z" fill="rgba(0,0,0,0.3)" class="part p2" />

    <!-- an open window -->
    <g class="part p3">
      <rect x="70" y="44" width="150" height="96" rx="6" fill="#1e1e20" stroke="rgba(255,255,255,0.08)" />
      <rect x="70" y="44" width="150" height="15" rx="6" fill="#2a2a2d" />
      <circle cx="208" cy="51.5" r="3" fill="#ef5a33" />
      <circle cx="198" cy="51.5" r="3" fill="#fec431" />
      <rect x="80" y="68" width="38" height="62" rx="3" fill="#2a2a2d" />
      <rect x="126" y="68" width="84" height="8" rx="3" fill="#ff7d2e" opacity="0.9" />
      <rect x="126" y="82" width="70" height="5" rx="2.5" fill="rgba(255,255,255,0.18)" />
      <rect x="126" y="92" width="78" height="5" rx="2.5" fill="rgba(255,255,255,0.12)" />
      <rect x="126" y="102" width="60" height="5" rx="2.5" fill="rgba(255,255,255,0.12)" />
      <rect x="126" y="116" width="36" height="12" rx="6" fill="#ff7d2e" />
    </g>

    {#if edition === "tyson"}
      <!-- Plasma: one panel at the bottom -->
      <g class="part p4">
        <rect x="10" y="172" width="300" height="16" fill="rgba(18,18,20,0.92)" />
        <circle cx="22" cy="180" r="5" fill="#ff7d2e" />
        {#each [0, 1, 2, 3] as n}
          <rect x={36 + n * 18} y="174" width="13" height="12" rx="2.5" fill={n === 0 ? "rgba(255,125,46,0.55)" : "rgba(255,255,255,0.16)"} />
        {/each}
        <rect x="248" y="177" width="6" height="6" rx="1.5" fill="rgba(255,255,255,0.3)" />
        <rect x="258" y="177" width="6" height="6" rx="1.5" fill="rgba(255,255,255,0.3)" />
        <rect x="272" y="176" width="30" height="8" rx="2" fill="rgba(255,255,255,0.22)" />
      </g>
    {:else if edition === "boro"}
      <!-- GNOME: a top bar with the clock in the middle, and the dash -->
      <g class="part p4">
        <rect x="10" y="8" width="300" height="11" fill="rgba(10,10,12,0.95)" />
        <rect x="16" y="11" width="22" height="5" rx="2.5" fill="rgba(255,255,255,0.3)" />
        <rect x="145" y="11" width="30" height="5" rx="2.5" fill="rgba(255,255,255,0.45)" />
        <rect x="280" y="11" width="24" height="5" rx="2.5" fill="rgba(255,255,255,0.3)" />
      </g>
      <g class="part p5">
        <rect x="104" y="163" width="112" height="20" rx="8" fill="rgba(18,18,20,0.85)" stroke="rgba(255,255,255,0.08)" />
        {#each [0, 1, 2, 3, 4] as n}
          <rect x={112 + n * 20} y="167" width="13" height="12" rx="3.5" fill={n === 0 ? "#ff7d2e" : n === 4 ? "rgba(255,255,255,0.35)" : "rgba(255,255,255,0.2)"} />
        {/each}
      </g>
    {:else}
      <!-- Xfce: a thin panel on top and a dock at the bottom -->
      <g class="part p4">
        <rect x="10" y="8" width="300" height="10" fill="rgba(18,18,20,0.92)" />
        <rect x="15" y="10.5" width="14" height="5" rx="1.5" fill="#ff7d2e" />
        <rect x="34" y="10.5" width="34" height="5" rx="1.5" fill="rgba(255,255,255,0.18)" />
        <rect x="276" y="10.5" width="28" height="5" rx="1.5" fill="rgba(255,255,255,0.3)" />
      </g>
      <g class="part p5">
        <rect x="114" y="166" width="92" height="18" rx="6" fill="rgba(18,18,20,0.85)" stroke="rgba(255,255,255,0.08)" />
        {#each [0, 1, 2, 3] as n}
          <rect x={121 + n * 21} y="169" width="14" height="12" rx="3" fill={n === 1 ? "#fec431" : "rgba(255,255,255,0.22)"} />
        {/each}
      </g>
    {/if}
  </g>
</svg>

<style>
  svg {
    display: block;
    width: 100%;
    height: auto;
    filter: drop-shadow(0 20px 40px rgba(255, 106, 18, 0.25));
  }
  .animated .part {
    opacity: 0;
    animation: partIn 0.8s cubic-bezier(0.2, 0.9, 0.2, 1) both;
  }
  .animated .p1 {
    animation-delay: 0.3s;
  }
  .animated .p2 {
    animation-delay: 0.45s;
  }
  .animated .p3 {
    animation-delay: 0.8s;
    transform-origin: 145px 92px;
    animation-name: windowIn;
  }
  .animated .p4 {
    animation-delay: 1.1s;
  }
  .animated .p5 {
    animation-delay: 1.3s;
  }
  @keyframes partIn {
    from {
      opacity: 0;
      transform: translateY(8px);
    }
    to {
      opacity: 1;
      transform: none;
    }
  }
  @keyframes windowIn {
    from {
      opacity: 0;
      transform: scale(0.85);
    }
    to {
      opacity: 1;
      transform: none;
    }
  }
</style>

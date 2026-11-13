<script lang="ts">
  import { onMount, onDestroy } from "svelte";
  import { locale } from "svelte-i18n";
  import { textsFor } from "./texts";
  import EditionArt from "./EditionArt.svelte";

  // The presentation shown while Soplos installs (DESIGN.md, section 9), a
  // section of its own with its texts in ./texts. Scenes, in a loop:
  //   the edition being installed; Soplos Linux, the distribution; the three
  //   editions; two scenes on each edition's desktop (the one being installed
  //   first); the programs of this edition one by one; the family.
  // About 3.5 minutes a loop: an installation takes some 5 on a fast machine.
  // The style comes from soplos-artwork/presentación; sizes follow the box it
  // is drawn in (container units), not the screen.

  /** "tyron", "tyson" or "boro": the edition being installed. */
  export let edition = "";
  /** The boot loader chosen: GRUB Editor only shows with GRUB. */
  export let bootloader = "";

  interface App {
    slug: string;
    name: string;
    a1: string;
    a2: string;
    /** Only in these editions. */
    only?: string[];
    /** Only with this boot loader. */
    needs?: string;
  }

  const APPS: App[] = [
    { slug: "welcome", name: "Welcome", a1: "#ffd23f", a2: "#ff7a12" },
    { slug: "store", name: "Store", a1: "#ffc21f", a2: "#ff6a12" },
    { slug: "theme-manager", name: "Theme Manager", a1: "#ffb52e", a2: "#ff5a14", only: ["tyron"] },
    { slug: "look-manager", name: "Look Manager", a1: "#ffb52e", a2: "#ff6a12", only: ["tyson"] },
    { slug: "layout-switcher", name: "Layout Switcher", a1: "#ffd23f", a2: "#ff6a12", only: ["boro"] },
    { slug: "gaming-panel", name: "Gaming Panel", a1: "#ffd23f", a2: "#ff4a14" },
    { slug: "appimage-manager", name: "AppImage Manager", a1: "#ffb52e", a2: "#ff7a12" },
    { slug: "webapp-manager", name: "WebApp Manager", a1: "#ffc21f", a2: "#ff6a12" },
    { slug: "kernel-installer", name: "Kernel Installer", a1: "#ffc21f", a2: "#ff7a12" },
    { slug: "grub-editor", name: "GRUB Editor", a1: "#ff9a1f", a2: "#ff5a14", needs: "grub" },
    { slug: "plymouth-manager", name: "Plymouth Manager", a1: "#ffb52e", a2: "#ff5a14" },
    { slug: "repo-selector", name: "Repo Selector", a1: "#ffc21f", a2: "#ff6a12" },
    { slug: "system-service", name: "System Service", a1: "#ffc21f", a2: "#ff6a12" },
    { slug: "sys-cleaner", name: "Sys Cleaner", a1: "#ffa21f", a2: "#ff4a14" },
  ];

  /** The whole family for the last scene: all sixteen, of every edition. */
  const FAMILY: { slug: string; name: string }[] = [
    ...APPS.map(({ slug, name }) => ({ slug, name })),
    { slug: "welcome-live", name: "Welcome Live" },
    { slug: "sli", name: "Soplos Linux Installer" },
  ];

  const EDITIONS = [
    { id: "tyron", name: "Tyron", desktop: "Xfce" },
    { id: "tyson", name: "Tyson", desktop: "KDE Plasma" },
    { id: "boro", name: "Boro", desktop: "GNOME" },
  ] as const;

  $: t = textsFor($locale);
  $: current = EDITIONS.find((e) => e.id === edition);
  $: apps = APPS.filter((a) => (!a.only || a.only.includes(edition)) && (!a.needs || a.needs === bootloader));

  type EditionId = (typeof EDITIONS)[number]["id"];
  type Scene =
    | { kind: "intro" }
    | { kind: "soplos" }
    | { kind: "family" }
    | { kind: "edition"; id: EditionId; n: number }
    | { kind: "app"; app: App }
    | { kind: "outro" };

  /** The edition being installed first, then the other two. */
  $: editionOrder = [...EDITIONS].sort((a, b) => Number(b.id === edition) - Number(a.id === edition));
  $: scenes = [
    { kind: "intro" },
    { kind: "soplos" },
    { kind: "family" },
    ...editionOrder.flatMap((e) => [0, 1].map((n) => ({ kind: "edition", id: e.id, n }))),
    ...apps.map((app) => ({ kind: "app", app })),
    { kind: "outro" },
  ] as Scene[];
  const key = (s: Scene) => (s.kind === "app" ? s.app.slug : s.kind === "edition" ? `${s.id}-${s.n}` : s.kind);

  const EDITION_COLORS: Record<EditionId, [string, string]> = {
    tyron: ["#ffc21f", "#ff7d2e"],
    tyson: ["#ffb52e", "#ef5a33"],
    boro: ["#ffd23f", "#ff9a1f"],
  };

  // The background stays; only the elements change. A scene first plays
  // its way out (letters fall, the art flies off, the chips shrink) and then
  // the next one plays its way in.
  const SECONDS = 9;
  const EXIT_MS = 750;
  let index = 0;
  let leaving = false;
  let timer: ReturnType<typeof setInterval>;
  let exitTimer: ReturnType<typeof setTimeout>;

  function advance(next: number) {
    if (leaving || next === index) return;
    leaving = true;
    exitTimer = setTimeout(() => {
      index = next;
      leaving = false;
    }, EXIT_MS);
  }
  function restart() {
    clearInterval(timer);
    timer = setInterval(() => advance((index + 1) % scenes.length), SECONDS * 1000);
  }
  function go(i: number) {
    advance(i);
    restart();
  }
  onMount(restart);
  onDestroy(() => {
    clearInterval(timer);
    clearTimeout(exitTimer);
  });

  /** The colors the background glides to for each scene. */
  function colorsOf(scene: Scene | undefined): [string, string] {
    if (scene?.kind === "app") return [scene.app.a1, scene.app.a2];
    if (scene?.kind === "edition") return EDITION_COLORS[scene.id];
    if (scene?.kind === "soplos" || scene?.kind === "outro") return ["#ffc21f", "#ff4a14"];
    return ["#ffc21f", "#ff6a12"];
  }
  $: [b1, b2] = colorsOf(scenes[index]);

  /** Where the light falls: behind the art of the scene, the second glow in
   * the opposite corner. [x, y of the main glow, x of the second one]. */
  function glowOf(scene: Scene | undefined): [number, number, number] {
    if (!scene || scene.kind === "family" || scene.kind === "outro") return [50, 42, 50];
    const left = scene.kind === "soplos" || (scene.kind === "edition" && scene.n === 1) || (scene.kind === "app" && layoutOf(scene.app) === "left");
    return left ? [24, 50, 90] : [76, 50, 10];
  }
  $: [gx, gy, hx] = glowOf(scenes[index]);

  /** The programs take turns in two compositions, so no two in a row look alike. */
  const LAYOUTS = ["right", "left"] as const;
  const layoutOf = (app: App) => LAYOUTS[apps.indexOf(app) % LAYOUTS.length];

  /** Splits a title into words of letters for the rising animation. */
  const letters = (text: string) => text.split(" ").map((word) => word.split(""));
</script>

<div class="stage" style="--b1: {b1}; --b2: {b2}; --gx: {gx}%; --gy: {gy}%; --hx: {hx}%;">
  <div class="grid-bg"></div>
  <div class="bg"></div>

  {#each scenes as scene, i (key(scene))}
    {#if i === index}
      {#if scene.kind === "intro"}
        <section class="scene" class:leaving style="--a1: #ffc21f; --a2: #ff6a12;">
          <div class="copy">
            <p class="kicker"><span class="dot"></span>{t.kicker}<span class="bar"></span></p>
            <h3 class="title">
              {#each letters(`Soplos ${current?.name ?? "Linux"}`) as word, w}
                <span class="w">{#each word as l, k}<span class="l" style="--i: {w * 6 + k}">{l}</span>{/each}</span>
              {/each}
            </h3>
            {#if current}<p class="tag">{@html t.editions[current.id]}</p>{/if}
            <p class="desc">{t.introText}</p>
          </div>
          <div class="screen"><EditionArt edition={edition || "tyron"} /></div>
        </section>
      {:else if scene.kind === "soplos"}
        <section class="scene layout-left" class:leaving style="--a1: #ffc21f; --a2: #ff4a14;">
          <div class="copy">
            <p class="kicker"><span class="dot"></span>{t.soplos.cat}<span class="bar"></span></p>
            <h3 class="title">
              {#each letters("Soplos Linux") as word, w}
                <span class="w">{#each word as l, k}<span class="l" style="--i: {w * 6 + k}">{l}</span>{/each}</span>
              {/each}
            </h3>
            <p class="tag">{@html t.soplos.tag}</p>
            <p class="desc">{t.soplos.desc}</p>
            <ul class="chips">
              {#each t.soplos.chips as chip, c}<li style="--i: {c}">{chip}</li>{/each}
            </ul>
          </div>
          <div class="art">
            <div class="halo"></div>
            <div class="hoop r1"></div>
            <div class="hoop r2"></div>
            <div class="orbit"><i></i></div>
            <img class="icon logo" src="/assets/icons/soplos-logo.png" alt="Soplos Linux" />
          </div>
        </section>
      {:else if scene.kind === "family"}
        <section class="scene center" class:leaving style="--a1: #ffc21f; --a2: #ff6a12;">
          <div class="copy">
            <p class="kicker"><span class="dot"></span>{t.familyCat}<span class="bar"></span></p>
            <h3 class="title">
              {#each letters(t.familyTitle) as word, w}
                <span class="w">{#each word as l, k}<span class="l" style="--i: {w * 6 + k}">{l}</span>{/each}</span>
              {/each}
            </h3>
          </div>
          <div class="editions">
            {#each EDITIONS as e, n}
              <div class="edition" class:mine={e.id === edition} style="--i: {n}">
                {#if e.id === edition}<span class="installing">{t.installing}</span>{/if}
                <div class="thumb"><EditionArt edition={e.id} animated={false} /></div>
                <span class="ename">{e.name}</span>
                <span class="desktop">{e.desktop}</span>
                <span class="etext">{t.cards[e.id]}</span>
              </div>
            {/each}
          </div>
        </section>
      {:else if scene.kind === "edition"}
        {@const slide = t.editionSlides[scene.id][scene.n]}
        {@const [c1, c2] = EDITION_COLORS[scene.id]}
        <section class="scene {scene.n === 1 ? 'layout-left' : ''}" class:leaving style="--a1: {c1}; --a2: {c2};">
          <div class="copy">
            <p class="kicker"><span class="dot"></span>{slide.cat}<span class="bar"></span></p>
            <h3 class="title">
              {#each letters(EDITIONS.find((e) => e.id === scene.id)?.name ?? "") as word, w}
                <span class="w">{#each word as l, k}<span class="l" style="--i: {w * 6 + k}">{l}</span>{/each}</span>
              {/each}
            </h3>
            <p class="tag">{@html slide.tag}</p>
            <p class="desc">{slide.desc}</p>
            <ul class="chips">
              {#each slide.chips as chip, c}<li style="--i: {c}">{chip}</li>{/each}
            </ul>
          </div>
          <div class="screen"><EditionArt edition={scene.id} /></div>
        </section>
      {:else if scene.kind === "app"}
        <section class="scene layout-{layoutOf(scene.app)}" class:leaving style="--a1: {scene.app.a1}; --a2: {scene.app.a2};">
          <div class="copy">
            <p class="kicker"><span class="dot"></span>{t.apps[scene.app.slug]?.cat}<span class="bar"></span></p>
            <h3 class="title">
              {#each letters(scene.app.name) as word, w}
                <span class="w">{#each word as l, k}<span class="l" style="--i: {w * 6 + k}">{l}</span>{/each}</span>
              {/each}
            </h3>
            <p class="tag">{@html t.apps[scene.app.slug]?.tag}</p>
            <p class="desc">{t.apps[scene.app.slug]?.desc}</p>
            <ul class="chips">
              {#each t.apps[scene.app.slug]?.chips ?? [] as chip, c}<li style="--i: {c}">{chip}</li>{/each}
            </ul>
          </div>
          <div class="art">
            <div class="halo"></div>
            <div class="hoop r1"></div>
            <div class="hoop r2"></div>
            <div class="orbit"><i></i></div>
            <img class="icon" src="/assets/presentation/{scene.app.slug}.png" alt="" />
          </div>
        </section>
      {:else}
        <section class="scene center outro" class:leaving style="--a1: #ffc21f; --a2: #ff4a14;">
          <h3 class="title">
            {#each letters(t.outroTitle) as word, w}
              <span class="w">{#each word as l, k}<span class="l" style="--i: {w * 6 + k}">{l}</span>{/each}</span>
            {/each}
          </h3>
          <div class="wall">
            {#each FAMILY as app, a}
              <img src="/assets/presentation/{app.slug}.png" alt={app.name} title={app.name} style="--i: {a}" />
            {/each}
          </div>
          <p class="desc">{t.outroText}</p>
        </section>
      {/if}
    {/if}
  {/each}

  <div class="dots">
    {#each scenes as _scene, i}
      <button type="button" class:on={i === index} aria-label={String(i + 1)} on:click={() => go(i)} style="--t: {SECONDS}s"></button>
    {/each}
  </div>
</div>

<style>
  .stage {
    --fg: #f7efe4;
    --mut: #a8a196;
    --ease: cubic-bezier(0.2, 0.9, 0.2, 1);
    --pop: cubic-bezier(0.2, 1.25, 0.3, 1);
    position: relative;
    height: 100%;
    width: 100%;
    overflow: hidden;
    border-radius: 0.5rem;
    background: #09090c;
    color: var(--fg);
    container-type: size;
  }
  .grid-bg {
    position: absolute;
    inset: -2px;
    opacity: 0.55;
    pointer-events: none;
    background-image: linear-gradient(rgba(255, 255, 255, 0.035) 1px, transparent 1px),
      linear-gradient(90deg, rgba(255, 255, 255, 0.035) 1px, transparent 1px);
    background-size: 40px 40px;
    mask-image: radial-gradient(ellipse at 50% 50%, #000 20%, transparent 75%);
    -webkit-mask-image: radial-gradient(ellipse at 50% 50%, #000 20%, transparent 75%);
    animation: gridMove 18s linear infinite;
  }
  @keyframes gridMove {
    to {
      background-position: 40px 40px;
    }
  }

  .scene {
    position: absolute;
    inset: 0;
    display: grid;
    grid-template-columns: 1.15fr 0.85fr;
    align-items: center;
    gap: 3cqw;
    padding: 6cqh 6cqw 10cqh;
  }

  /* --- Compositions of the program scenes ------------------------------ */
  .layout-left {
    grid-template-columns: 0.85fr 1.15fr;
  }
  .layout-left .art,
  .layout-left .screen {
    order: -1;
  }
  .layout-left.leaving .screen {
    animation-name: artOutLeft;
  }
  .layout-left .icon {
    animation-name: iconInSide, float;
  }
  @keyframes iconInSide {
    0% {
      opacity: 0;
      transform: translateX(-60%) rotate(-40deg) scale(0.5);
    }
    100% {
      opacity: 1;
      transform: none;
    }
  }

  /* --- The way out: every element leaves before the next scene comes in -- */
  .leaving .kicker {
    animation: outUp 0.35s ease-in both;
  }
  .leaving .title .l {
    animation: fallOut 0.45s cubic-bezier(0.5, 0, 0.9, 0.4) both;
    animation-delay: calc(var(--i) * 0.018s);
  }
  .leaving .tag {
    animation: outLeft 0.45s ease-in 0.05s both;
  }
  .leaving .desc {
    animation: outLeft 0.45s ease-in 0.1s both;
  }
  .leaving .chips li {
    animation: shrinkOut 0.3s ease-in both;
    animation-delay: calc(var(--i) * 0.05s);
  }
  .leaving .art,
  .leaving .screen {
    animation: artOut 0.6s cubic-bezier(0.6, 0, 0.9, 0.5) both;
  }
  .layout-left.leaving .art {
    animation-name: artOutLeft;
  }
  .leaving .edition,
  .leaving .wall img {
    animation: shrinkOut 0.35s ease-in both;
    animation-delay: calc(var(--i) * 0.03s);
  }
  @keyframes outUp {
    from {
      opacity: 1;
      transform: none;
    }
    to {
      opacity: 0;
      transform: translateY(-16px);
    }
  }
  @keyframes fallOut {
    from {
      opacity: 1;
      transform: none;
    }
    to {
      opacity: 0;
      transform: translateY(110%) rotate(8deg);
    }
  }
  @keyframes outLeft {
    from {
      opacity: 1;
      transform: none;
    }
    to {
      opacity: 0;
      transform: translateX(-40px);
    }
  }
  @keyframes shrinkOut {
    from {
      opacity: 1;
      transform: none;
    }
    to {
      opacity: 0;
      transform: scale(0.4);
    }
  }
  @keyframes artOut {
    from {
      opacity: 1;
      transform: none;
    }
    to {
      opacity: 0;
      transform: translateX(30cqw) rotate(25deg) scale(0.6);
    }
  }
  @keyframes artOutLeft {
    from {
      opacity: 1;
      transform: none;
    }
    to {
      opacity: 0;
      transform: translateX(-30cqw) rotate(-25deg) scale(0.6);
    }
  }
  /* Larger than the stage on every side, so neither the drift nor the
     pulse ever brings its edge into view (it showed as a black band). The
     positions are mapped onto this larger box to land where they should. */
  .bg {
    position: absolute;
    inset: -15%;
    z-index: 0;
    pointer-events: none;
    background: radial-gradient(
        56cqw 56cqw at calc(11.54% + var(--gx) * 0.7692) calc(11.54% + var(--gy) * 0.7692),
        var(--b1),
        transparent 62%
      ),
      radial-gradient(44cqw 44cqw at calc(11.54% + var(--hx) * 0.7692) 94.6%, var(--b2), transparent 62%);
    opacity: 0.26;
    animation: bgPulse 7s ease-in-out infinite alternate, bgDrift 23s ease-in-out infinite alternate;
  }
  /* The glows never stand still: a slow drift on top of the pulse. */
  @keyframes bgDrift {
    0% {
      translate: 0 0;
    }
    33% {
      translate: 3cqw -2cqh;
    }
    66% {
      translate: -2cqw 3cqh;
    }
    100% {
      translate: 2cqw 1cqh;
    }
  }
  /* --b1 and --b2 are registered in styles/global.css, so the colors glide
     from one scene to the next instead of jumping. */
  .stage {
    transition:
      --b1 1.4s ease,
      --b2 1.4s ease,
      --gx 1.6s cubic-bezier(0.6, 0, 0.3, 1),
      --gy 1.6s cubic-bezier(0.6, 0, 0.3, 1),
      --hx 1.6s cubic-bezier(0.6, 0, 0.3, 1);
  }
  @keyframes bgPulse {
    to {
      transform: scale(1.08);
    }
  }
  .copy,
  .art,
  .title,
  .wall,
  .desc {
    position: relative;
    z-index: 1;
  }

  .kicker {
    display: inline-flex;
    align-items: center;
    gap: 0.6em;
    font-size: clamp(9px, 1.5cqw, 13px);
    font-weight: 700;
    letter-spacing: 0.28em;
    text-transform: uppercase;
    color: var(--a1);
    animation: fadeUp 0.7s var(--ease) 0.15s both;
  }
  .kicker .dot {
    width: 8px;
    height: 8px;
    border-radius: 50%;
    background: linear-gradient(135deg, var(--a1), var(--a2));
    box-shadow: 0 0 14px var(--a1);
  }
  .kicker .bar {
    width: 0;
    height: 2px;
    background: linear-gradient(90deg, var(--a1), transparent);
    animation: grow 1s var(--ease) 0.5s both;
  }
  @keyframes grow {
    to {
      width: 50px;
    }
  }

  .title {
    margin: 1.5cqh 0 2cqh;
    font-weight: 900;
    letter-spacing: -0.045em;
    line-height: 0.95;
    font-size: clamp(26px, 7.4cqw, 76px);
  }
  .title .w {
    display: inline-block;
    overflow: hidden;
    padding: 0.06em 0.04em 0.1em 0;
    margin-right: 0.16em;
    vertical-align: top;
  }
  .title .l {
    display: inline-block;
    transform: translateY(115%) rotate(6deg);
    opacity: 0;
    background: linear-gradient(180deg, #fff 10%, var(--a1) 70%, var(--a2));
    -webkit-background-clip: text;
    background-clip: text;
    color: transparent;
    animation: rise 0.95s var(--ease) both;
    animation-delay: calc(0.3s + var(--i) * 0.045s);
  }
  @keyframes rise {
    to {
      transform: none;
      opacity: 1;
    }
  }

  .tag {
    font-size: clamp(14px, 2.6cqw, 26px);
    font-weight: 600;
    line-height: 1.22;
    color: #fff3df;
    animation: fadeUp 0.9s var(--ease) 0.75s both;
  }
  .tag :global(b) {
    font-weight: 800;
    background: linear-gradient(90deg, var(--a1), var(--a2));
    -webkit-background-clip: text;
    background-clip: text;
    color: transparent;
  }
  .desc {
    margin-top: 1.4cqh;
    font-size: clamp(11px, 1.7cqw, 16px);
    line-height: 1.5;
    max-width: 36em;
    color: var(--mut);
    animation: fadeUp 0.9s var(--ease) 0.95s both;
  }
  @keyframes fadeUp {
    from {
      opacity: 0;
      transform: translateY(20px);
    }
    to {
      opacity: 1;
      transform: none;
    }
  }

  .chips {
    list-style: none;
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
    margin-top: 2.6cqh;
  }
  .chips li {
    padding: 0.5em 1em;
    border-radius: 999px;
    font-size: clamp(10px, 1.5cqw, 14px);
    font-weight: 600;
    color: #ffe9c9;
    border: 1px solid rgba(255, 194, 31, 0.45);
    background: rgba(255, 106, 18, 0.14);
    opacity: 0;
    transform: scale(0.6);
    animation: popIn 0.6s var(--pop) both;
    animation-delay: calc(1.05s + var(--i) * 0.11s);
  }
  @keyframes popIn {
    to {
      opacity: 1;
      transform: none;
    }
  }

  .art {
    display: grid;
    place-items: center;
    aspect-ratio: 1;
    width: min(100%, 62cqh);
    justify-self: center;
  }
  .art > * {
    grid-area: 1 / 1;
  }
  .icon {
    width: 82%;
    height: 82%;
    object-fit: contain;
    z-index: 3;
    filter: drop-shadow(0 24px 40px rgba(255, 106, 18, 0.55));
    animation: iconIn 1.25s var(--pop) 0.2s both, float 5.5s ease-in-out 1.5s infinite;
  }
  @keyframes iconIn {
    0% {
      opacity: 0;
      transform: scale(0.15) rotate(-28deg) translateY(60px);
    }
    60% {
      opacity: 1;
    }
    100% {
      opacity: 1;
      transform: none;
    }
  }
  @keyframes float {
    0%,
    100% {
      transform: translateY(0) rotate(0);
    }
    50% {
      transform: translateY(-2.2%) rotate(1.6deg);
    }
  }
  .halo {
    width: 74%;
    height: 74%;
    border-radius: 50%;
    z-index: 1;
    filter: blur(36px);
    opacity: 0;
    background: radial-gradient(circle, var(--a1), var(--a2) 55%, transparent 72%);
    animation: haloIn 1.4s ease 0.2s both, haloPulse 4s ease-in-out 1.6s infinite alternate;
  }
  @keyframes haloIn {
    to {
      opacity: 0.55;
    }
  }
  @keyframes haloPulse {
    from {
      transform: scale(0.92);
    }
    to {
      transform: scale(1.1);
    }
  }
  /* "hoop", not "ring": Tailwind has a .ring utility that draws its own
     blue ring on any element with that class. */
  .hoop {
    border-radius: 50%;
    border: 1.5px solid rgba(255, 194, 31, 0.4);
    z-index: 2;
    opacity: 0;
    transform: scale(0.3);
    animation: ringIn 1.1s var(--ease) 0.35s both;
  }
  .hoop.r1 {
    width: 94%;
    height: 94%;
  }
  .hoop.r2 {
    width: 112%;
    height: 112%;
    border-style: dashed;
    border-color: rgba(255, 106, 18, 0.45);
    animation: ringIn 1.1s var(--ease) 0.5s both, spin 40s linear 1.6s infinite;
  }
  @keyframes ringIn {
    to {
      opacity: 1;
      transform: scale(1);
    }
  }
  @keyframes spin {
    to {
      transform: rotate(360deg);
    }
  }
  .orbit {
    position: relative;
    width: 100%;
    height: 100%;
    border-radius: 50%;
    z-index: 4;
    animation: spin 9s linear 1.2s infinite;
  }
  .orbit i {
    position: absolute;
    top: -6px;
    left: 50%;
    width: 12px;
    height: 12px;
    margin-left: -6px;
    border-radius: 50%;
    background: linear-gradient(135deg, #fff, var(--a1));
    box-shadow: 0 0 18px 4px var(--a1);
  }

  .center {
    grid-template-columns: 1fr;
    text-align: center;
    justify-items: center;
    align-content: center;
  }
  .center .desc {
    margin-left: auto;
    margin-right: auto;
  }
  .outro {
    gap: 3cqh;
  }
  .screen {
    position: relative;
    z-index: 1;
    width: 100%;
    max-width: 62cqh;
    justify-self: center;
    animation: fadeUp 1s var(--ease) 0.2s both;
  }
  .thumb {
    width: 88%;
    margin-bottom: 0.6cqh;
  }
  .icon.logo {
    width: 64%;
    height: 64%;
  }

  .editions {
    position: relative;
    z-index: 1;
    display: grid;
    grid-template-columns: repeat(3, 1fr);
    gap: 2cqw;
    width: min(88cqw, 760px);
    margin-top: 2cqh;
  }
  .edition {
    position: relative;
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 0.4em;
    padding: 3cqh 1.6cqw 2.6cqh;
    border-radius: 16px;
    border: 1px solid rgba(255, 255, 255, 0.1);
    background: rgba(255, 255, 255, 0.035);
    opacity: 0;
    transform: translateY(24px) scale(0.92);
    animation: popIn 0.7s var(--pop) both;
    animation-delay: calc(0.6s + var(--i) * 0.15s);
  }
  .edition.mine {
    border-color: rgba(255, 194, 31, 0.7);
    background: rgba(255, 106, 18, 0.16);
    box-shadow: 0 0 40px rgba(255, 106, 18, 0.35);
  }
  .installing {
    position: absolute;
    top: -0.8em;
    padding: 0.25em 0.8em;
    border-radius: 999px;
    font-size: clamp(9px, 1.2cqw, 12px);
    font-weight: 800;
    letter-spacing: 0.14em;
    text-transform: uppercase;
    color: #1a0d03;
    background: linear-gradient(90deg, var(--a1), var(--a2));
  }
  .ename {
    font-size: clamp(18px, 3.6cqw, 34px);
    font-weight: 900;
    letter-spacing: -0.03em;
    background: linear-gradient(180deg, #fff 20%, var(--a1));
    -webkit-background-clip: text;
    background-clip: text;
    color: transparent;
  }
  .desktop {
    font-size: clamp(10px, 1.5cqw, 14px);
    font-weight: 700;
    letter-spacing: 0.2em;
    text-transform: uppercase;
    color: var(--a1);
  }
  .etext {
    font-size: clamp(10px, 1.5cqw, 14px);
    line-height: 1.4;
    color: var(--mut);
  }
  .wall {
    display: grid;
    grid-template-columns: repeat(8, 1fr);
    gap: clamp(6px, 1.6cqw, 18px);
    width: min(88cqw, 720px);
  }
  .wall img {
    width: 100%;
    aspect-ratio: 1;
    object-fit: contain;
    opacity: 0;
    transform: scale(0.2) rotate(-20deg);
    filter: drop-shadow(0 10px 16px rgba(255, 120, 20, 0.25));
    animation: popIn 0.7s var(--pop) both;
    animation-delay: calc(0.4s + var(--i) * 0.06s);
  }

  .dots {
    position: absolute;
    left: 0;
    right: 0;
    bottom: 3cqh;
    z-index: 5;
    display: flex;
    justify-content: center;
    gap: 6px;
  }
  .dots button {
    width: 18px;
    height: 4px;
    border-radius: 2px;
    background: rgba(255, 255, 255, 0.18);
    position: relative;
    overflow: hidden;
  }
  .dots button.on::after {
    content: "";
    position: absolute;
    inset: 0;
    background: linear-gradient(90deg, #ffc21f, #ff4a14);
    transform-origin: left;
    animation: fill var(--t) linear both;
  }
  @keyframes fill {
    from {
      transform: scaleX(0);
    }
  }
</style>

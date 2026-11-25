<script lang="ts">
  import { setContext, onMount, type ComponentType } from "svelte";
  import { getCurrentWindow } from "@tauri-apps/api/window";
  import { getVersion } from "@tauri-apps/api/app";
  import { _ } from "svelte-i18n";
  import { api } from "./lib/api";
  import { STEPS, current, reached, canContinue, plan } from "./lib/wizard";
  import Sidebar from "./components/Sidebar.svelte";
  import WindowControls from "./components/WindowControls.svelte";
  import AboutDialog from "./components/AboutDialog.svelte";
  import StepHeader from "./components/StepHeader.svelte";
  import WizardNav from "./components/WizardNav.svelte";
  import { installPercent, installDone } from "./lib/install";
  import { altHeld, mnemonicActions, keyOf } from "./lib/mnemonics";
  import Modal from "./components/Modal.svelte";
  import Welcome from "./views/Welcome.svelte";
  import Location from "./views/Location.svelte";
  import Bootloader from "./views/Bootloader.svelte";
  import Kernel from "./views/Kernel.svelte";
  import Disk from "./views/Disk.svelte";
  import User from "./views/User.svelte";
  import Summary from "./views/Summary.svelte";
  import Install from "./views/Install.svelte";
  import Finished from "./views/Finished.svelte";
  import { Sparkles, Globe, Power, Cpu, HardDrive, User as UserIcon, ClipboardList, Download, Flag } from "lucide-svelte";

  const icons: Record<string, ComponentType> = {
    welcome: Sparkles,
    location: Globe,
    bootloader: Power,
    kernel: Cpu,
    disk: HardDrive,
    user: UserIcon,
    summary: ClipboardList,
    install: Download,
    finished: Flag,
  };

  const views: Record<string, ComponentType> = {
    welcome: Welcome,
    location: Location,
    bootloader: Bootloader,
    kernel: Kernel,
    disk: Disk,
    user: User,
    summary: Summary,
    install: Install,
    finished: Finished,
  };

  $: steps = STEPS.map((id) => ({ id, icon: icons[id], label: $_(`steps.${id}.label`) }));
  $: stepId = STEPS[$current];

  let appVersion = "";
  let edition = "";

  // With `decorations` off in tauri.conf.json (and `transparent` on) the system
  // draws no frame: the window is one card with rounded corners and the buttons
  // of the window float at its top right.
  let frameless = false;
  let maximized = false;

  onMount(() => {
    let unlisten: (() => void) | undefined;
    (async () => {
      try {
        appVersion = await getVersion();
      } catch {
        appVersion = "";
      }
      try {
        const env = await api.invoke<{ edition: string; firmware: "uefi" | "bios" }>("get_environment_info");
        edition = env.edition;
        plan.update((p) => ({ ...p, firmware: env.firmware }));
      } catch {
        edition = "";
      }
      // A newer installer in the repository: offered before anything else.
      api
        .invoke<string | null>("check_update")
        .then((version) => {
          if (version && STEPS[$current] === "welcome") updateVersion = version;
        })
        .catch(() => {});
      try {
        const win = getCurrentWindow();
        frameless = !(await win.isDecorated());
        if (frameless) {
          maximized = await win.isMaximized();
          unlisten = await win.onResized(async () => {
            maximized = await win.isMaximized();
          });
        }
      } catch {
        frameless = false;
      }
    })();
    return () => unlisten?.();
  });

  // Once the installation starts nothing can be revisited or closed.
  $: locked = stepId === "install" || stepId === "finished";

  function goTo(index: number) {
    if (locked) return;
    current.set(index);
    canContinue.set(true);
    document.getElementById("main-scroll")?.scrollTo({ top: 0 });
  }

  function next() {
    const index = $current + 1;
    reached.update((r) => Math.max(r, index));
    goTo(index);
  }

  function back() {
    if ($current > 0) goTo($current - 1);
  }

  // The update of the installer itself (commands/update.rs): it installs the
  // new version, opens it and closes this window.
  let updateVersion: string | null = null;
  let updating = false;
  let updateFailed = false;

  async function applyUpdate() {
    updating = true;
    updateFailed = false;
    try {
      await api.invoke("apply_update");
    } catch {
      updating = false;
      updateFailed = true;
    }
  }

  function skipUpdate() {
    if (!updating) updateVersion = null;
  }

  // The last word before any disk is touched.
  let confirmOpen = false;

  function startInstall() {
    confirmOpen = false;
    const index = STEPS.indexOf("install");
    reached.set(index);
    current.set(index);
  }

  function restart() {
    api.invoke("reboot").catch(() => getCurrentWindow().close());
  }

  let aboutOpen = false;
  setContext("openAbout", () => (aboutOpen = true));

  // F1 -> About, Ctrl+Q -> quit (not while the installation runs).
  // The installer can be used without the mouse, as Calamares. Holding Alt
  // underlines a letter of each button and Alt + that letter presses it
  // (lib/mnemonics.ts). Besides:
  //   Alt+Right / Alt+Left  next / back       Alt+1..7  a step already reached
  //   F1 about, Ctrl+Q quit (not while the installation runs). Enter and
  //   Escape answer the dialogs (Modal.svelte).
  // The list is in README.md.
  function handleKeydown(e: KeyboardEvent) {
    if (e.key === "Alt") {
      altHeld.set(true);
      return;
    }
    if (e.key === "F1") {
      e.preventDefault();
      aboutOpen = true;
      return;
    }
    if (e.ctrlKey && (e.key === "q" || e.key === "Q") && (stepId !== "install" || $installDone)) {
      e.preventDefault();
      getCurrentWindow().close();
      return;
    }
    if (!e.altKey || e.ctrlKey || confirmOpen || aboutOpen) return;
    const collecting = !locked;
    let handled = true;
    if (e.key === "ArrowRight" && collecting) {
      if (!$canContinue) return;
      if (stepId === "summary") confirmOpen = true;
      else next();
    } else if (e.key === "ArrowLeft" && collecting) {
      back();
    } else if (/^[1-7]$/.test(e.key) && collecting && Number(e.key) - 1 <= $reached) {
      goTo(Number(e.key) - 1);
    } else if ($mnemonicActions[keyOf(e.key)]) {
      $mnemonicActions[keyOf(e.key)]();
    } else {
      handled = false;
    }
    if (handled) e.preventDefault();
  }

  function handleKeyup(e: KeyboardEvent) {
    if (e.key === "Alt") altHeld.set(false);
  }


</script>

<svelte:window on:keydown={handleKeydown} on:keyup={handleKeyup} on:blur={() => altHeld.set(false)} />

<div class="relative flex h-screen overflow-hidden bg-dark {frameless && !maximized ? 'window-card' : ''}">
  <Sidebar {steps} current={$current} reached={$reached} {locked} onSelect={goTo} {appVersion} {edition} />

  {#if frameless}
    <div class="absolute right-2 top-[5px] z-40"><WindowControls {maximized} closeDisabled={stepId === "install" && !$installDone} /></div>
  {/if}

  <div class="flex-1 flex flex-col overflow-hidden min-w-0">
    {#if frameless}
      <div class="h-7 shrink-0" data-tauri-drag-region></div>
    {/if}
    <div id="main-scroll" class="content-scroll flex-1 overflow-y-auto p-7 {frameless ? 'pt-0' : ''}">
      <!-- The map of the location step, the partitions of the disk step, the
           summary and the installation need the whole height: no banner. -->
      {#if !["location", "disk", "summary", "install"].includes(stepId)}
        <StepHeader
          title={$_(`steps.${stepId}.title`)}
          subtitle={$_(`steps.${stepId}.subtitle`)}
          icon={icons[stepId]}
        />
      {/if}
      <svelte:component this={views[stepId]} />
    </div>

    <WizardNav
      step={stepId}
      isFirst={$current === 0}
      canContinue={$canContinue}
      onBack={back}
      onNext={next}
      onInstall={() => (confirmOpen = true)}
      onRestart={restart}
      onReturn={() => current.set(STEPS.indexOf("summary"))}
      onClose={() => getCurrentWindow().close()}
    />

    <!-- The thin bar of the other Soplos apps: filled to the percentage, or
         sliding while it is not known. -->
    {#if stepId === "install"}
      <div class="h-1.5 bg-surface-2 overflow-hidden shrink-0 relative">
        {#if $installDone}
          <div class="h-full {$installDone.success ? 'bg-primary' : 'bg-danger'}" style="width: 100%"></div>
        {:else if $installPercent !== null && $installPercent > 0}
          <div class="h-full bg-primary transition-[width] duration-500" style="width: {$installPercent}%"></div>
        {:else}
          <div class="absolute inset-y-0 bg-primary progress-fill"></div>
        {/if}
      </div>
    {/if}
  </div>

  {#if confirmOpen}
    <Modal
      variant="danger"
      title={$_("confirm.title")}
      confirmLabel={$_("confirm.accept")}
      onClose={() => (confirmOpen = false)}
      onConfirm={startInstall}
    >
      {$_("confirm.body")}
    </Modal>
  {/if}

  {#if updateVersion}
    <Modal
      variant="confirm"
      title={$_("update.title")}
      confirmLabel={$_("update.accept")}
      cancelLabel={$_("update.later")}
      confirmDisabled={updating}
      onClose={skipUpdate}
      onConfirm={applyUpdate}
    >
      {#if updating}
        {$_("update.working")}
      {:else if updateFailed}
        {$_("update.failed")}
      {:else}
        {$_("update.body", { values: { version: updateVersion } })}
      {/if}
    </Modal>
  {/if}

  {#if aboutOpen}
    <AboutDialog onClose={() => (aboutOpen = false)} />
  {/if}
</div>

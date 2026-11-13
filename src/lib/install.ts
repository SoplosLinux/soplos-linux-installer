import { writable } from "svelte/store";

// The state of the installation while the helper runs. The install view
// fills it from the helper's events; the bottom bar (WizardNav) and the thin
// progress bar under it, as in the other Soplos apps, read it.

export interface InstallDone {
  success: boolean;
  message: string;
  dryRun: boolean;
}

/** The id of the step running ("partition", "format"...), "" before the first. */
export const installStep = writable<string>("");

/** 0 to 100; null while it is not known (the bar slides instead). */
export const installPercent = writable<number | null>(null);

/** How it ended; null while it runs. */
export const installDone = writable<InstallDone | null>(null);

/** The log instead of the presentation. */
export const showLog = writable<boolean>(false);

export function resetInstall() {
  installStep.set("");
  installPercent.set(null);
  installDone.set(null);
  showLog.set(false);
}

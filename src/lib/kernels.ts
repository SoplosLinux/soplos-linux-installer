/**
 * The Soplos kernels as the window names them (views/Kernel.svelte,
 * views/Summary.svelte). A flavor is the package name without
 * "linux-soplos" and the level: "" is stock, "lts-zen" is Zen in its LTS
 * branch.
 */

/** The families Soplos builds, in the order they are offered. */
export const FAMILY_NAMES: Record<string, string> = {
  "": "Stock",
  "bore-ntsync": "BORE + NTsync",
  zen: "Zen",
  rt: "RT",
  x3d: "X3D",
};

export const isLts = (flavor: string) => flavor === "lts" || flavor.startsWith("lts-");
export const familyOf = (flavor: string) => (flavor === "lts" ? "" : flavor.replace(/^lts-/, ""));
export const flavorFor = (family: string, lts: boolean) => (lts ? (family ? `lts-${family}` : "lts") : family);

/** A family the window does not know yet: "foo-bar" -> "FOO + BAR". */
export function familyName(family: string): string {
  if (family in FAMILY_NAMES) return FAMILY_NAMES[family];
  return family
    .split("-")
    .map((w) => (w.length <= 4 ? w.toUpperCase() : w[0].toUpperCase() + w.slice(1)))
    .join(" + ");
}

/** "lts-zen" -> "Zen LTS". */
export const kernelName = (flavor: string) => `${familyName(familyOf(flavor))}${isLts(flavor) ? " LTS" : ""}`;

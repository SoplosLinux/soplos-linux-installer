// The texts of the presentation, one file per language in ./texts. They live
// apart from the installer's own strings: the presentation is a section of
// its own that can be changed without touching the installer.
//
// The language is the installer's, which starts as the one of the system
// (svelte-i18n reads it from the session) and follows the choice made on
// the welcome step. A language without its own file falls back to English.

import es from "./texts/es.json";
import en from "./texts/en.json";
import de from "./texts/de.json";
import fr from "./texts/fr.json";
import it from "./texts/it.json";
import pt from "./texts/pt.json";
import ro from "./texts/ro.json";
import ru from "./texts/ru.json";

export interface AppText {
  cat: string;
  /** May hold <b>…</b> for the highlighted part. */
  tag: string;
  desc: string;
  chips: string[];
}

/** The shape of every file, with the programs looked up by their slug. */
export type Texts = Omit<typeof es, "apps"> & { apps: Record<string, AppText> };

const TEXTS: Record<string, Texts> = { es, en, de, fr, it, pt, ro, ru };

export function textsFor(locale: string | null | undefined): Texts {
  const base = (locale ?? "").split(/[-_]/)[0].toLowerCase();
  return TEXTS[base] ?? en;
}

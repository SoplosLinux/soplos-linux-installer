// What the location step works with, and how it guesses the language and the
// keyboard from the place: the first choice the user sees should already be
// the right one most of the time.

export interface Zone {
  id: string;
  country: string;
  lat: number;
  lon: number;
}

export interface Layout {
  name: string;
  description: string;
  variants: { name: string; description: string }[];
}

/** "America/Argentina/Buenos_Aires" -> "Buenos Aires". */
export function cityOf(id: string): string {
  return (id.split("/").pop() ?? id).replace(/_/g, " ");
}

/** "es_ES.UTF-8" -> "es-ES"; "ca_ES.UTF-8@valencia" -> "ca-ES". */
export function bcp47(locale: string): string {
  return locale.split(/[.@]/)[0].replace("_", "-");
}

/** The name of a glibc locale in the language of the installer. */
export function localeName(locale: string, ui: string): string {
  const modifier = locale.split("@")[1];
  let name = locale;
  try {
    name = new Intl.DisplayNames([ui], { type: "language" }).of(bcp47(locale)) ?? locale;
  } catch {
    /* an unusual code keeps its raw name */
  }
  return modifier ? `${name} — ${modifier}` : name;
}

export function countryName(code: string, ui: string): string {
  try {
    return new Intl.DisplayNames([ui], { type: "region" }).of(code) ?? code;
  } catch {
    return code;
  }
}

/** The zone the map starts on when nothing better is known. */
export const FALLBACK_ZONE: Record<string, string> = {
  en: "Europe/London",
  es: "Europe/Madrid",
  de: "Europe/Berlin",
  fr: "Europe/Paris",
  it: "Europe/Rome",
  pt: "Europe/Lisbon",
  ro: "Europe/Bucharest",
  ru: "Europe/Moscow",
};

const MAIN_LOCALE: Record<string, string> = {
  en: "en_US",
  es: "es_ES",
  de: "de_DE",
  fr: "fr_FR",
  it: "it_IT",
  pt: "pt_PT",
  ro: "ro_RO",
  ru: "ru_RU",
};

/** The language of the system: the one of the installer, in its variant for the country when there is one. */
export function guessLanguage(ui: string, country: string, locales: string[]): string {
  for (const candidate of [`${ui}_${country}.UTF-8`, `${MAIN_LOCALE[ui] ?? "en_US"}.UTF-8`, "en_US.UTF-8"]) {
    if (locales.includes(candidate)) return candidate;
  }
  return locales[0] ?? "en_US.UTF-8";
}

/** The main language of countries whose code is not their language's. */
const COUNTRY_LANGUAGE: Record<string, string> = {
  US: "en", GB: "en", IE: "en", AU: "en", NZ: "en", CA: "en", ZA: "en", IN: "en",
  AT: "de", CH: "de", BE: "nl", BR: "pt", AR: "es", MX: "es", CL: "es", CO: "es",
  PE: "es", VE: "es", UY: "es", EC: "es", BO: "es", PY: "es", CR: "es", CU: "es",
  DO: "es", GT: "es", HN: "es", NI: "es", PA: "es", SV: "es", PR: "es",
  UA: "uk", BY: "be", KZ: "kk", CN: "zh", TW: "zh", JP: "ja", KR: "ko", GR: "el",
  SE: "sv", DK: "da", NO: "nb", CZ: "cs", IL: "he", IR: "fa", VN: "vi",
};

/**
 * Dates, numbers and money: those of the country. In the language of the
 * installer when the country has it (es_MX), else in the country's own
 * language (en installer in Spain: es_ES, not the first *_ES found, which
 * is Aragonese).
 */
export function guessFormats(ui: string, country: string, locales: string[], language: string): string {
  const candidates = [
    `${ui}_${country}.UTF-8`,
    `${country.toLowerCase()}_${country}.UTF-8`,
    `${COUNTRY_LANGUAGE[country] ?? ""}_${country}.UTF-8`,
  ];
  for (const candidate of candidates) {
    if (locales.includes(candidate)) return candidate;
  }
  return locales.find((l) => l.endsWith(`_${country}.UTF-8`)) ?? language;
}

const LATAM = ["AR", "BO", "CL", "CO", "CR", "CU", "DO", "EC", "GT", "HN", "MX", "NI", "PA", "PE", "PR", "PY", "SV", "UY", "VE"];

// Countries whose code is a layout of another script or language, or that
// type on another country's layout.
const KEYBOARD_OF: Record<string, string> = {
  US: "us",
  GB: "gb",
  IE: "gb",
  CA: "us",
  AU: "us",
  NZ: "us",
  IN: "us",
  ZA: "us",
  PH: "us",
  SG: "us",
};

/** The keyboard of the country, else of the language, else US English. */
export function guessKeyboard(country: string, ui: string, layouts: Layout[]): string {
  const has = (name: string) => layouts.some((l) => l.name === name);
  if (LATAM.includes(country)) return "latam";
  if (KEYBOARD_OF[country]) return KEYBOARD_OF[country];
  const own = country.toLowerCase();
  if (has(own)) return own;
  if (has(ui)) return ui;
  return "us";
}

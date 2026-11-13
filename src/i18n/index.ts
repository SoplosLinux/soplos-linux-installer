import { register, init, getLocaleFromNavigator, locale as currentLocale } from "svelte-i18n";

export const SUPPORTED_LOCALES = ["en", "es", "de", "fr", "it", "pt", "ro", "ru"] as const;
export type SupportedLocale = (typeof SUPPORTED_LOCALES)[number];

export const LOCALE_NAMES: Record<SupportedLocale, string> = {
  en: "English",
  es: "Español",
  de: "Deutsch",
  fr: "Français",
  it: "Italiano",
  pt: "Português",
  ro: "Română",
  ru: "Русский",
};

register("en", () => import("./locales/en.json"));
register("es", () => import("./locales/es.json"));
register("de", () => import("./locales/de.json"));
register("fr", () => import("./locales/fr.json"));
register("it", () => import("./locales/it.json"));
register("pt", () => import("./locales/pt.json"));
register("ro", () => import("./locales/ro.json"));
register("ru", () => import("./locales/ru.json"));

function isSupported(locale: string | null | undefined): locale is SupportedLocale {
  return !!locale && (SUPPORTED_LOCALES as readonly string[]).includes(locale);
}

// The installer starts in the language of the live session (WebKitGTK's
// navigator.language reflects the session's LANG), down to the base language
// for regional variants like "es-ES" or "pt-BR". Nothing is stored: the
// language chosen on the first step only lasts while the installer runs.
function initialLocale(): SupportedLocale {
  const fromNavigator = getLocaleFromNavigator();
  const base = fromNavigator?.split("-")[0];
  if (isSupported(base)) return base;
  return "en";
}

export function setLocale(locale: SupportedLocale) {
  currentLocale.set(locale);
}

// init()'s return type is `void | Promise<void>` (it is synchronous only
// when every registered locale is already loaded, which never applies here
// since each one is a dynamic import), so it is wrapped to give main.ts a
// real Promise to wait on.
export const i18nReady = Promise.resolve(
  init({
    fallbackLocale: "en",
    initialLocale: initialLocale(),
  }),
);

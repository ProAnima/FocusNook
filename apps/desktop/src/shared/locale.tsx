import { useEffect, useState, type ReactNode } from "react";
import { commands, type Locale } from "./commands";
import { LocaleContext } from "./locale-context";
import { LOCALES, translate } from "./translations";

const DEFAULT_LOCALE: Locale = "ru";

function previewLocale(): Locale | null {
  if (!import.meta.env.DEV) return null;
  const value = new URLSearchParams(window.location.search).get("localePreview");
  return LOCALES.find((locale) => locale === value) ?? null;
}

export function LocaleProvider({ children }: { children: ReactNode }) {
  const preview = previewLocale();
  const [locale, setLocaleState] = useState<Locale>(preview ?? DEFAULT_LOCALE);

  useEffect(() => {
    if (preview) return;
    let cancelled = false;
    commands.settings
      .getLocale()
      .then((saved) => {
        if (!cancelled && saved) setLocaleState(saved);
      })
      .catch(() => {
        // Вне Tauri (browser-preview) язык не сохраняется — остаёмся на дефолтном.
      });
    return () => {
      cancelled = true;
    };
  }, [preview]);

  function setLocale(next: Locale) {
    setLocaleState(next);
    void commands.settings.setLocale(next);
  }

  return (
    <LocaleContext.Provider value={{ locale, t: (key) => translate(locale, key), setLocale }}>
      {children}
    </LocaleContext.Provider>
  );
}

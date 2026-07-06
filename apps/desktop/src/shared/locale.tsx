import { useEffect, useState, type ReactNode } from "react";
import { commands, type Locale } from "./commands";
import { LocaleContext } from "./locale-context";
import { translate } from "./translations";

const DEFAULT_LOCALE: Locale = "ru";

export function LocaleProvider({ children }: { children: ReactNode }) {
  const [locale, setLocaleState] = useState<Locale>(DEFAULT_LOCALE);

  useEffect(() => {
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
  }, []);

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

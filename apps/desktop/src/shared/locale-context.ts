import { createContext } from "react";
import type { Locale } from "./commands";
import { translate, type TranslationKey } from "./translations";

export type LocaleContextValue = {
  locale: Locale;
  t: (key: TranslationKey) => string;
  setLocale: (locale: Locale) => void;
};

const DEFAULT_LOCALE: Locale = "ru";

// Дефолт — не null: компоненты вне LocaleProvider (тесты, будущие изолированные
// окна) получают рабочий перевод на ru вместо падения по throw, как у useTheme.
// Тут это уместно (в отличие от темы) — "нет провайдера" не значит баг разметки
// приложения, а просто "показываем текст на языке по умолчанию".
export const LocaleContext = createContext<LocaleContextValue>({
  locale: DEFAULT_LOCALE,
  t: (key) => translate(DEFAULT_LOCALE, key),
  setLocale: () => {},
});

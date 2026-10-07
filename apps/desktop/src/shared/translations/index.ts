import type { Locale } from "../commands";
import { de } from "./de";
import { en } from "./en";
import { es } from "./es";
import { fr } from "./fr";
import { hi } from "./hi";
import { ja } from "./ja";
import { ko } from "./ko";
import { pt } from "./pt";
import { ru } from "./ru";
import type { TranslationKey } from "./types";
import { zh } from "./zh";

export type { TranslationKey } from "./types";

const DICTIONARIES: Record<Locale, Record<TranslationKey, string>> = {
  ru,
  en,
  es,
  de,
  fr,
  pt,
  zh,
  ja,
  ko,
  hi,
};

export const LOCALES: readonly Locale[] = ["ru", "en", "es", "de", "fr", "pt", "zh", "ja", "ko", "hi"];

export const LOCALE_LABELS: Record<Locale, string> = {
  ru: "Русский",
  en: "English",
  es: "Español",
  de: "Deutsch",
  fr: "Français",
  pt: "Português",
  zh: "简体中文",
  ja: "日本語",
  ko: "한국어",
  hi: "हिन्दी",
};

// BCP-47 теги для Intl.DateTimeFormat — держим рядом с остальными locale-
// таблицами, а не в reminderPresets.ts, чтобы новый язык добавлялся в одном месте.
export const INTL_LOCALE_TAG: Record<Locale, string> = {
  ru: "ru-RU",
  en: "en-US",
  es: "es-ES",
  de: "de-DE",
  fr: "fr-FR",
  pt: "pt-BR",
  zh: "zh-CN",
  ja: "ja-JP",
  ko: "ko-KR",
  hi: "hi-IN",
};

// Обычная функция (не хук) — нужна и вне React-дерева компонентов
// (reminderPresets.ts генерирует подписи пресетов не из компонента).
export function translate(locale: Locale, key: TranslationKey): string {
  return DICTIONARIES[locale][key];
}

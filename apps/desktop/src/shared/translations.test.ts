import { describe, expect, it } from "vitest";
import type { Locale } from "./commands";
import { LOCALES, translate, type TranslationKey } from "./translations";

const ACCOUNT_KEYS: TranslationKey[] = [
  "account.welcome",
  "account.setupHint",
  "account.choose",
  "account.passwordHint",
  "account.confirmPassword",
  "account.add",
  "account.logout",
  "account.error",
  "account.syncHint",
  "account.syncConsent",
  "account.syncAdvanced",
  "settings.accountSyncTitle",
  "settings.accountSyncHint",
];

const THEME_COPY_KEYS: TranslationKey[] = [
  "settings.themeLightDesc",
  "settings.themeDarkDesc",
  "settings.liveThemes",
  "settings.themeAuroraDesc",
  "settings.themeSunsetDesc",
  "settings.themeOceanDesc",
  "settings.themeForestDesc",
  "settings.themeGlacierDesc",
  "settings.themeNebulaDesc",
  "settings.themeEmberDesc",
  "settings.themePrismDesc",
];

describe("account localization", () => {
  it("has native account copy for every advertised non-English locale", () => {
    const localized = LOCALES.filter((locale): locale is Exclude<Locale, "en"> => locale !== "en");
    for (const locale of localized) {
      for (const key of ACCOUNT_KEYS) {
        expect(translate(locale, key), `${locale}:${key}`).not.toBe(translate("en", key));
      }
    }
  });

  it("has native theme descriptions for every advertised non-English locale", () => {
    const localized = LOCALES.filter((locale): locale is Exclude<Locale, "en"> => locale !== "en");
    for (const locale of localized) {
      for (const key of THEME_COPY_KEYS) {
        expect(translate(locale, key), `${locale}:${key}`).not.toBe(translate("en", key));
      }
    }
  });
});

import type { ru } from "./ru";

export type Dictionary = Record<keyof typeof ru, string>;

export type TranslationKey = keyof typeof ru;

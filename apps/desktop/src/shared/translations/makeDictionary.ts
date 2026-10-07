import { en } from "./en";
import type { Dictionary } from "./types";

export function makeDictionary(overrides: Partial<Dictionary>): Dictionary {
  return { ...en, ...overrides };
}

import { describe, expect, it } from "vitest";
// @ts-expect-error Vitest runs in Node; the frontend intentionally does not ship Node typings.
import { readFileSync } from "node:fs";
import { THEME_OPTIONS } from "./themeCatalog";

const css = readFileSync("src/theme.css", "utf8");

const REQUIRED_TOKENS = ["surface-gradient", "text-primary", "text-secondary", "text-tertiary", "accent", "accent-contrast"];

function luminance(hex: string) {
  const channels = [1, 3, 5].map((index) => Number.parseInt(hex.slice(index, index + 2), 16) / 255);
  const linear = channels.map((value) => value <= 0.03928 ? value / 12.92 : ((value + 0.055) / 1.055) ** 2.4);
  return 0.2126 * linear[0] + 0.7152 * linear[1] + 0.0722 * linear[2];
}

function contrast(first: string, second: string) {
  const values = [luminance(first), luminance(second)];
  return (Math.max(...values) + 0.05) / (Math.min(...values) + 0.05);
}

function themeBlock(mode: string) {
  const selectorStart = css.indexOf(`[data-theme="${mode}"]`);
  const blockStart = css.indexOf("{", selectorStart);
  const blockEnd = css.indexOf("}", blockStart);
  if (selectorStart < 0 || blockStart < 0 || blockEnd < 0) throw new Error(`missing CSS block for ${mode}`);
  return css.slice(blockStart + 1, blockEnd);
}

function color(block: string, token: string) {
  const match = block.match(new RegExp(`--${token}:[^#]*(#[0-9a-f]{6})`, "i"));
  if (!match) throw new Error(`missing color token ${token}`);
  return match[1];
}

describe("theme accessibility", () => {
  for (const option of THEME_OPTIONS.filter(({ mode }) => mode !== "system")) {
    it(`${option.mode} defines a complete readable palette`, () => {
      const block = themeBlock(option.mode);
      for (const token of REQUIRED_TOKENS) expect(block).toContain(`--${token}:`);
      const background = color(block, "surface-gradient");
      expect(contrast(background, color(block, "text-primary"))).toBeGreaterThanOrEqual(7);
      expect(contrast(background, color(block, "text-secondary"))).toBeGreaterThanOrEqual(4.5);
      expect(contrast(background, color(block, "text-tertiary"))).toBeGreaterThanOrEqual(4.5);
      expect(contrast(color(block, "accent"), color(block, "accent-contrast"))).toBeGreaterThanOrEqual(4.5);
    });
  }
});

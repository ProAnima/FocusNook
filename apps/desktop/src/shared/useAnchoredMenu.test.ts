import { describe, expect, it } from "vitest";
import { placeMenu } from "./useAnchoredMenu";

const rect = (left: number, top: number, width: number, height: number) =>
  ({ left, top, width, height, right: left + width, bottom: top + height }) as DOMRect;

describe("placeMenu", () => {
  const viewport = { width: 400, height: 300 };

  it("opens below the trigger, aligned to its right edge", () => {
    expect(placeMenu(rect(200, 20, 30, 20), rect(0, 0, 100, 80), viewport)).toEqual({ left: 130, top: 45 });
  });

  it("flips above the trigger when there is no room below", () => {
    expect(placeMenu(rect(200, 250, 30, 20), rect(0, 0, 100, 80), viewport)).toEqual({ left: 130, top: 165 });
  });

  it("keeps the menu inside the viewport horizontally", () => {
    expect(placeMenu(rect(0, 20, 30, 20), rect(0, 0, 100, 80), viewport).left).toBe(8);
    expect(placeMenu(rect(390, 20, 30, 20), rect(0, 0, 100, 80), viewport).left).toBe(292);
  });
});

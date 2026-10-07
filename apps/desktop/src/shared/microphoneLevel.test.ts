import { describe, expect, it } from "vitest";
import { peakLevel } from "./microphoneLevel";

describe("peakLevel", () => {
  it("is zero for silence", () => {
    expect(peakLevel(new Uint8Array([128, 128, 128]))).toBe(0);
  });

  it("scales the largest deviation from the midpoint", () => {
    expect(peakLevel(new Uint8Array([128, 128 + 36, 128 - 10]))).toBeCloseTo(0.5);
  });

  it("clamps to 1", () => {
    expect(peakLevel(new Uint8Array([0, 255]))).toBe(1);
  });
});

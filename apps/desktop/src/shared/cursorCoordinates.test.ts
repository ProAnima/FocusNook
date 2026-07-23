import { describe, expect, it } from "vitest";
import { physicalCursorToClient } from "./cursorCoordinates";

describe("physicalCursorToClient", () => {
  it("converts physical screen coordinates at scaled DPI", () => {
    expect(
      physicalCursorToClient({ x: 450, y: 300 }, { x: 150, y: 75 }, 1.5),
    ).toEqual({ x: 200, y: 150 });
  });

  it("falls back to an unscaled conversion for an invalid ratio", () => {
    expect(
      physicalCursorToClient({ x: 450, y: 300 }, { x: 150, y: 75 }, 0),
    ).toEqual({ x: 300, y: 225 });
  });
});

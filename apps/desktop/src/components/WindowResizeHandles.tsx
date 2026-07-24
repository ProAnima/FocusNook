import type { KeyboardEvent, PointerEvent } from "react";
import { commands, type ResizeDirection } from "../shared/commands";
import { useLocale } from "../shared/useLocale";

const EDGE_DIRECTIONS: ResizeDirection[] = [
  "North",
  "NorthEast",
  "East",
  "South",
  "SouthWest",
  "West",
  "NorthWest",
];

function startResize(direction: ResizeDirection, event: PointerEvent) {
  event.preventDefault();
  event.stopPropagation();
  void commands.overlay.startResize(direction);
}

function keyboardResize(event: KeyboardEvent<HTMLButtonElement>) {
  const step = event.shiftKey ? 48 : 16;
  const delta = {
    ArrowLeft: [-step, 0],
    ArrowRight: [step, 0],
    ArrowUp: [0, -step],
    ArrowDown: [0, step],
  }[event.key];
  if (!delta) return;
  event.preventDefault();
  void commands.overlay.resizeBy(delta[0], delta[1]);
}

export function WindowResizeHandles() {
  const { t } = useLocale();
  return (
    <>
      {EDGE_DIRECTIONS.map((direction) => (
        <div
          aria-hidden="true"
          className={`window-resize-edge resize-${direction.toLowerCase()}`}
          data-cursor-hit-area="true"
          key={direction}
          onPointerDown={(event) => startResize(direction, event)}
        />
      ))}
      <button
        className="window-resize-grip"
        data-cursor-hit-area="true"
        type="button"
        aria-label={t("header.resize")}
        title={t("header.resize")}
        onKeyDown={keyboardResize}
        onPointerDown={(event) => startResize("SouthEast", event)}
      >
        <span aria-hidden="true" />
      </button>
    </>
  );
}

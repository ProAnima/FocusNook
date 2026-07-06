import { useEffect } from "react";
import { commands } from "./commands";

const HIT_TEST_INTERVAL_MS = 70;
const INTERACTIVE_SURFACE_SELECTOR = [
  ".overlay-shell",
  ".note-folder-rail",
  ".note-folder-create",
  ".note-folder-mobile-layer",
  "[data-cursor-hit-area='true']",
].join(", ");

function hitsInteractiveSurface(x: number, y: number) {
  if (x < 0 || y < 0 || x > window.innerWidth || y > window.innerHeight) return false;
  return Boolean(document.elementFromPoint(x, y)?.closest(INTERACTIVE_SURFACE_SELECTOR));
}

export function useDesktopCursorPassthrough(enabled: boolean) {
  useEffect(() => {
    if (!enabled) return;

    let disposed = false;
    let lastIgnore: boolean | null = null;

    async function setIgnore(nextIgnore: boolean) {
      if (lastIgnore === nextIgnore) return;
      lastIgnore = nextIgnore;
      try {
        await commands.overlay.setIgnoreCursorEvents(nextIgnore);
      } catch {
        // Browser tests and non-Tauri previews do not expose this native window API.
      }
    }

    async function refreshHitTest() {
      try {
        const cursor = await commands.overlay.getCursorClientPosition();
        if (disposed) return;
        await setIgnore(!hitsInteractiveSurface(cursor.x, cursor.y));
      } catch {
        await setIgnore(false);
      }
    }

    const intervalId = window.setInterval(refreshHitTest, HIT_TEST_INTERVAL_MS);
    window.addEventListener("mousemove", refreshHitTest, { passive: true });
    window.addEventListener("mouseleave", refreshHitTest, { passive: true });
    void refreshHitTest();

    return () => {
      disposed = true;
      window.clearInterval(intervalId);
      window.removeEventListener("mousemove", refreshHitTest);
      window.removeEventListener("mouseleave", refreshHitTest);
      void commands.overlay.setIgnoreCursorEvents(false).catch(() => {});
    };
  }, [enabled]);
}

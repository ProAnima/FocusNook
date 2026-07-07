import { useCallback, useEffect, useState } from "react";
import { commands, type ShortcutInfo } from "./commands";

export type { ShortcutInfo };

function fallbackIsDesktop() {
  return window.innerWidth >= 720;
}

export function useLayerToggle() {
  const [front, setFront] = useState(true);
  const [shortcutInfo, setShortcutInfo] = useState<ShortcutInfo | null>(null);
  const [isDesktop, setIsDesktop] = useState(fallbackIsDesktop);

  const toggleLayer = useCallback(() => {
    commands.overlay.toggle().catch(() => {
      setFront((prev) => !prev);
    });
  }, []);

  useEffect(() => {
    commands.overlay
      .getShortcutStatus()
      .then(setShortcutInfo)
      .catch(() => {
        // Browser preview has no global shortcut state.
      });

    commands.overlay
      .isDesktop()
      .then(setIsDesktop)
      .catch(() => {
        setIsDesktop(fallbackIsDesktop());
      });

    const unlistenChanged = commands.overlay.onLayerChanged(setFront);
    return () => {
      void unlistenChanged.then((fn) => fn());
    };
  }, []);

  return { front, toggleLayer, shortcutInfo, isDesktop };
}

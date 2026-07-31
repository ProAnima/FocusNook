import { useCallback, useEffect, useState } from "react";
import { commands, type ShortcutInfo } from "./commands";

export type { ShortcutInfo };

function fallbackIsDesktop() {
  return window.innerWidth >= 720;
}

function previewIsDesktop(): boolean | null {
  if (!import.meta.env.DEV) return null;
  const value = new URLSearchParams(window.location.search).get("platformPreview");
  if (value === "desktop") return true;
  if (value === "mobile") return false;
  return null;
}

export function useLayerToggle() {
  const platformPreview = previewIsDesktop();
  const [front, setFront] = useState(true);
  const [shortcutInfo, setShortcutInfo] = useState<ShortcutInfo | null>(null);
  const [isDesktop, setIsDesktop] = useState(platformPreview ?? fallbackIsDesktop);

  const toggleLayer = useCallback(() => {
    commands.overlay.toggle().catch(() => {
      setFront((prev) => !prev);
    });
  }, []);

  useEffect(() => {
    if (platformPreview !== null) return;
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
  }, [platformPreview]);

  return { front, toggleLayer, shortcutInfo, isDesktop };
}

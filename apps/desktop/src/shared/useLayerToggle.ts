import { useCallback, useEffect, useState } from "react";
import { commands, type ShortcutInfo } from "./commands";

export type { ShortcutInfo };

export function useLayerToggle() {
  const [front, setFront] = useState(true);
  const [shortcutInfo, setShortcutInfo] = useState<ShortcutInfo | null>(null);

  const toggleLayer = useCallback(() => {
    commands.overlay.toggle().catch(() => {
      // Вне Tauri (например, в browser-preview) команда недоступна — переключаем UI сами.
      setFront((prev) => !prev);
    });
  }, []);

  useEffect(() => {
    commands.overlay
      .getShortcutStatus()
      .then(setShortcutInfo)
      .catch(() => {
        // Вне Tauri статус хоткея недоступен — просто не показываем секцию.
      });

    const unlistenChanged = commands.overlay.onLayerChanged(setFront);
    return () => {
      void unlistenChanged.then((fn) => fn());
    };
  }, []);

  return { front, toggleLayer, shortcutInfo };
}

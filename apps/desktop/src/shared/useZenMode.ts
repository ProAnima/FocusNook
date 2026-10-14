import { useEffect, useRef, useState } from "react";
import { commands } from "./commands";

export function useZenMode() {
  const [active, setActive] = useState(false);
  const changing = useRef(false);
  const interacted = useRef(false);
  useEffect(() => {
    let cancelled = false;
    commands.overlay.getZenMode().then((enabled) => {
      if (!cancelled && !interacted.current) setActive(enabled);
    }).catch(() => {});
    return () => { cancelled = true; };
  }, []);
  async function toggle() {
    if (changing.current) return;
    interacted.current = true;
    changing.current = true;
    try {
      await commands.overlay.setZenMode(!active);
      setActive(!active);
    } catch {
      // Browser preview has no native window; real failures preserve the current UI.
      if (!("__TAURI_INTERNALS__" in window)) setActive(!active);
    } finally {
      changing.current = false;
    }
  }
  return { active, toggle };
}

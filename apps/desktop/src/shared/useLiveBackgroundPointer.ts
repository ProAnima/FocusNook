import { useEffect } from "react";
import { isLiveTheme, type ResolvedTheme } from "./theme-context";

// Лёгкий параллакс "живого" фона по курсору. Пишем прямо в CSS-переменные на
// documentElement (как data-theme в theme.tsx), а не через React state — иначе
// каждое движение мыши гоняло бы рендер всего дерева ради двух чисел, нужных
// только CSS (см. .overlay-shell::before/::after в App.css).
export function useLiveBackgroundPointer(effective: ResolvedTheme) {
  useEffect(() => {
    if (!isLiveTheme(effective)) return;
    if (window.matchMedia("(prefers-reduced-motion: reduce)").matches) return;

    const root = document.documentElement;
    let frame: number | null = null;

    function handleMove(event: PointerEvent) {
      if (frame !== null) return;
      frame = requestAnimationFrame(() => {
        frame = null;
        root.style.setProperty("--pointer-x", (event.clientX / window.innerWidth - 0.5).toFixed(3));
        root.style.setProperty("--pointer-y", (event.clientY / window.innerHeight - 0.5).toFixed(3));
      });
    }

    window.addEventListener("pointermove", handleMove);
    return () => {
      window.removeEventListener("pointermove", handleMove);
      if (frame !== null) cancelAnimationFrame(frame);
      root.style.removeProperty("--pointer-x");
      root.style.removeProperty("--pointer-y");
    };
  }, [effective]);
}

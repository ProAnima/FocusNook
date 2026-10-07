import { useEffect } from "react";

/** Пока мобильная панель открыта: блокирует прокрутку страницы и закрывает её по Escape. */
export function useMobileSheetLock(active: boolean, onEscape: () => void) {
  useEffect(() => {
    if (!active) return;
    const previousOverflow = document.body.style.overflow;
    document.body.style.overflow = "hidden";
    function handleKeyDown(event: KeyboardEvent) {
      if (event.key === "Escape") onEscape();
    }
    window.addEventListener("keydown", handleKeyDown);
    return () => {
      document.body.style.overflow = previousOverflow;
      window.removeEventListener("keydown", handleKeyDown);
    };
  }, [active, onEscape]);
}

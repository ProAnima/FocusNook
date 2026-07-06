import { useEffect, type RefObject } from "react";

// Общий примитив для попапов/дропдаунов (сейчас — ProfileSwitcher, дальше
// пригодится для любого другого floating-меню без переиспечи той же логики).
// Раздел 21 ТЗ, "keyboard navigation": закрытие по Escape — почти всегда
// ожидаемое поведение для dismissible-попапа наравне с кликом снаружи, так
// что это в том же хуке, а не отдельная опция, о которой каждый вызывающий
// код должен не забыть.
export function useOutsideClick(ref: RefObject<HTMLElement | null>, onOutside: () => void) {
  useEffect(() => {
    function handleClick(event: MouseEvent) {
      if (ref.current && !ref.current.contains(event.target as Node)) {
        onOutside();
      }
    }
    function handleKeyDown(event: KeyboardEvent) {
      if (event.key === "Escape") onOutside();
    }
    document.addEventListener("mousedown", handleClick);
    document.addEventListener("keydown", handleKeyDown);
    return () => {
      document.removeEventListener("mousedown", handleClick);
      document.removeEventListener("keydown", handleKeyDown);
    };
  }, [ref, onOutside]);
}

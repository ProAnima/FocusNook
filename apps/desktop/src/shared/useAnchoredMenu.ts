import { useEffect, useLayoutEffect, useRef, useState } from "react";

const VIEWPORT_GAP = 8;
const TRIGGER_GAP = 5;

/** Позиция меню у правого края триггера: снизу, а если не помещается — сверху, в пределах окна. */
export function placeMenu(trigger: DOMRect, menu: DOMRect, viewport: { width: number; height: number }) {
  const left = Math.min(Math.max(VIEWPORT_GAP, trigger.right - menu.width), viewport.width - menu.width - VIEWPORT_GAP);
  const below = trigger.bottom + TRIGGER_GAP;
  const top =
    below + menu.height <= viewport.height - VIEWPORT_GAP ? below : Math.max(VIEWPORT_GAP, trigger.top - menu.height - TRIGGER_GAP);
  return { left, top };
}

/**
 * Состояние выпадающего меню, отрисованного порталом: позиционирование по
 * триггеру и закрытие по клику снаружи, Escape, resize и прокрутке.
 * `layoutKey` — значение, при изменении которого позицию нужно пересчитать
 * (например, число пунктов).
 */
export function useAnchoredMenu(layoutKey: unknown) {
  const [open, setOpen] = useState(false);
  const [position, setPosition] = useState({ left: 0, top: 0 });
  const triggerRef = useRef<HTMLButtonElement>(null);
  const menuRef = useRef<HTMLDivElement>(null);

  useLayoutEffect(() => {
    if (!open || !triggerRef.current || !menuRef.current) return;
    const viewport = { width: window.innerWidth, height: window.innerHeight };
    setPosition(placeMenu(triggerRef.current.getBoundingClientRect(), menuRef.current.getBoundingClientRect(), viewport));
  }, [open, layoutKey]);

  useEffect(() => {
    if (!open) return;
    function closeOutside(event: MouseEvent) {
      const target = event.target as Node;
      if (!triggerRef.current?.contains(target) && !menuRef.current?.contains(target)) setOpen(false);
    }
    function closeOnEscape(event: KeyboardEvent) {
      if (event.key === "Escape") setOpen(false);
    }
    const close = () => setOpen(false);
    window.addEventListener("mousedown", closeOutside);
    window.addEventListener("keydown", closeOnEscape);
    window.addEventListener("resize", close);
    window.addEventListener("scroll", close, true);
    return () => {
      window.removeEventListener("mousedown", closeOutside);
      window.removeEventListener("keydown", closeOnEscape);
      window.removeEventListener("resize", close);
      window.removeEventListener("scroll", close, true);
    };
  }, [open]);

  return { open, setOpen, position, triggerRef, menuRef };
}

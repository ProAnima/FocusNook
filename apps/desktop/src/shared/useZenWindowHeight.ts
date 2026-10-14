import { useLayoutEffect, useRef } from "react";
import { commands } from "./commands";

export function zenContentHeight(card: number, button: number, header: number, gap: number, padding: number) {
  return Math.min(180, Math.max(72, Math.ceil(card + button + header + gap + padding + 2)));
}

/** Fit the native window to intrinsic card height; cap long text so it remains scrollable. */
export function useZenWindowHeight(contentKey: string) {
  const viewRef = useRef<HTMLElement>(null);
  useLayoutEffect(() => {
    const view = viewRef.current;
    const body = view?.parentElement;
    const header = body?.parentElement?.querySelector<HTMLElement>(".zen-drag-zone");
    const card = view?.querySelector<HTMLElement>(".plan-list, .zen-finished");
    const button = view?.querySelector<HTMLElement>(".zen-next");
    if (!view || !body || !header || !card) return;
    let lastHeight = 0;
    function update() {
      if (!view || !body || !header || !card) return;
      const style = getComputedStyle(body);
      const padding = (parseFloat(style.paddingTop) || 0) + (parseFloat(style.paddingBottom) || 0);
      const gap = button ? parseFloat(getComputedStyle(view).rowGap) || 0 : 0;
      const height = zenContentHeight(card.offsetHeight, button?.offsetHeight ?? 0, header.offsetHeight, gap, padding);
      const actualHeight = view.closest<HTMLElement>(".overlay-shell")?.offsetHeight ?? height;
      if (height === lastHeight && Math.abs(actualHeight - height) <= 1) return;
      lastHeight = height;
      void commands.overlay.resizeZenWindow(height).catch(() => {});
    }
    update();
    if (typeof ResizeObserver === "undefined") return;
    const observer = new ResizeObserver(update);
    observer.observe(card);
    observer.observe(view);
    if (button) observer.observe(button);
    return () => observer.disconnect();
  }, [contentKey]);
  return viewRef;
}

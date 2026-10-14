import { useLayoutEffect, useRef, useState } from "react";

/** Measure the unwrapped title so switching layout never causes an oscillating measurement. */
export function usePlanItemLayout(title: string, status: string) {
  const rowRef = useRef<HTMLLIElement>(null);
  const measureRef = useRef<HTMLSpanElement>(null);
  const [stacked, setStacked] = useState(false);
  useLayoutEffect(() => {
    const row = rowRef.current;
    const measure = measureRef.current;
    if (!row || !measure) return;
    function update() {
      if (!row || !measure) return;
      const style = getComputedStyle(row);
      const parts = [".plan-checkbox", ".plan-item-actions", ".plan-progress"]
        .map((selector) => row.querySelector<HTMLElement>(selector))
        .filter((element) => element !== null);
      const controls = parts.reduce((total, element) => total + element.getBoundingClientRect().width, 0);
      const spacing = (parseFloat(style.columnGap) || 0) * parts.length;
      const padding = (parseFloat(style.paddingLeft) || 0) + (parseFloat(style.paddingRight) || 0);
      setStacked(measure.getBoundingClientRect().width + controls + spacing + padding > row.clientWidth);
    }
    update();
    if (typeof ResizeObserver === "undefined") return;
    const observer = new ResizeObserver(update);
    observer.observe(row);
    observer.observe(measure);
    const actions = row.querySelector(".plan-item-actions");
    if (actions) observer.observe(actions);
    return () => observer.disconnect();
  }, [title, status]);
  return { rowRef, measureRef, stacked };
}

import { useCallback, useEffect, useRef, useState, type KeyboardEvent, type MouseEvent, type PointerEvent } from "react";

const HOLD_TO_CONFIRM_MS = 900;

export function useHoldToConfirm(onConfirm: () => void) {
  const [holding, setHolding] = useState(false);
  const timerRef = useRef<number | null>(null);

  const cancel = useCallback(() => {
    if (timerRef.current !== null) {
      window.clearTimeout(timerRef.current);
      timerRef.current = null;
    }
    setHolding(false);
  }, []);

  const start = useCallback(() => {
    cancel();
    setHolding(true);
    timerRef.current = window.setTimeout(() => {
      timerRef.current = null;
      setHolding(false);
      onConfirm();
    }, HOLD_TO_CONFIRM_MS);
  }, [cancel, onConfirm]);

  const confirmFromKeyboard = useCallback(() => {
    cancel();
    onConfirm();
  }, [cancel, onConfirm]);

  useEffect(() => cancel, [cancel]);

  return {
    holding,
    cancel,
    buttonProps: {
      onBlur: cancel,
      onContextMenu: (event: MouseEvent) => event.preventDefault(),
      onKeyDown: (event: KeyboardEvent) => {
        if (event.key === "Enter" || event.key === " ") {
          event.preventDefault();
          confirmFromKeyboard();
        }
      },
      onPointerCancel: cancel,
      onPointerDown: (event: PointerEvent) => {
        if (event.button !== 0) return;
        event.currentTarget.setPointerCapture?.(event.pointerId);
        start();
      },
      onPointerLeave: cancel,
      onPointerUp: cancel,
    },
  };
}

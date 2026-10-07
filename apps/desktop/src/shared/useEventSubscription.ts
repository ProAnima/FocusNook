import { useEffect, useRef } from "react";

type Subscribe = (handler: () => void) => Promise<() => void>;

/**
 * Подписывает `handler` на событие на время жизни компонента. `handler` читается
 * через ref, поэтому его можно передавать инлайн — подписка пересоздаётся только
 * при смене `subscribe`. Если компонент размонтировался раньше, чем подписка
 * оформилась, слушатель снимается сразу после её завершения, а не остаётся висеть.
 */
export function useEventSubscription(subscribe: Subscribe, handler: () => void) {
  const handlerRef = useRef(handler);
  useEffect(() => {
    handlerRef.current = handler;
  });

  useEffect(() => {
    let disposed = false;
    let unlisten: (() => void) | null = null;
    subscribe(() => handlerRef.current())
      .then((cleanup) => {
        if (disposed) cleanup();
        else unlisten = cleanup;
      })
      .catch(() => {});
    return () => {
      disposed = true;
      unlisten?.();
    };
  }, [subscribe]);
}

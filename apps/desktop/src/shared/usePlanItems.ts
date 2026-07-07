import { useCallback, useEffect, useState } from "react";
import { commands, type PlanItem } from "./commands";

export function usePlanItems(planDate: string, autoRollOver = false) {
  const [items, setItems] = useState<PlanItem[]>([]);
  const [loadedDate, setLoadedDate] = useState<string | null>(null);

  const refresh = useCallback((withRollOver = autoRollOver) => {
    let cancelled = false;
    const ready = withRollOver
      ? commands.planItems.rollOverPending(planDate).catch(() => 0)
      : Promise.resolve(0);
    ready
      .then(() => commands.planItems.list(planDate))
      .then((nextItems) => {
        if (!cancelled) setItems(nextItems);
      })
      .catch(() => {
        if (!cancelled) setItems([]);
      })
      .finally(() => {
        if (!cancelled) setLoadedDate(planDate);
      });
    return () => {
      cancelled = true;
    };
  }, [autoRollOver, planDate]);

  useEffect(() => refresh(), [refresh]);

  useEffect(() => {
    let unlisten: (() => void) | null = null;
    commands.serverSync
      .onCompleted(() => {
        refresh(false);
      })
      .then((cleanup) => {
        unlisten = cleanup;
      })
      .catch(() => {});
    return () => unlisten?.();
  }, [refresh]);

  const addItem = useCallback(async (title: string) => {
    const created = await commands.planItems.create(title, planDate).catch(() => null);
    if (created) setItems((prev) => [...prev, created]);
  }, [planDate]);

  const toggleDone = useCallback(async (id: string) => {
    const updated = await commands.planItems.toggleDone(id).catch(() => null);
    if (updated) {
      setItems((prev) => prev.map((item) => (item.id === id ? updated : item)));
    }
  }, []);

  const cycleProgress = useCallback(async (id: string) => {
    const updated = await commands.planItems.cycleProgress(id).catch(() => null);
    if (updated) {
      setItems((prev) => prev.map((item) => (item.id === id ? updated : item)));
    }
  }, []);

  const toggleDeferred = useCallback(async (id: string) => {
    const updated = await commands.planItems.toggleDeferred(id).catch(() => null);
    if (updated) {
      setItems((prev) => prev.map((item) => (item.id === id ? updated : item)));
    }
  }, []);

  const moveToDate = useCallback(async (id: string, targetDate: string) => {
    const updated = await commands.planItems.moveToDate(id, targetDate).catch(() => null);
    if (updated) {
      setItems((prev) =>
        updated.planDate === planDate
          ? prev.map((item) => (item.id === id ? updated : item))
          : prev.filter((item) => item.id !== id),
      );
    }
  }, [planDate]);

  const deleteItem = useCallback(async (id: string) => {
    const previous = items;
    setItems((prev) => prev.filter((item) => item.id !== id));
    await commands.planItems.delete(id).catch(() => setItems(previous));
  }, [items]);

  return {
    items,
    loaded: loadedDate === planDate,
    addItem,
    toggleDone,
    cycleProgress,
    toggleDeferred,
    moveToDate,
    deleteItem,
  };
}

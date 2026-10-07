import { useCallback, useEffect, useState, type Dispatch, type SetStateAction } from "react";
import { commands, type PlanItem } from "./commands";
import { useEventSubscription } from "./useEventSubscription";

/** Загрузка списка дня: опциональный перенос незавершённого, повторная загрузка после синхронизации. */
function usePlanItemsData(planDate: string, autoRollOver: boolean) {
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
  const refreshWithoutRollOver = useCallback(() => void refresh(false), [refresh]);
  useEventSubscription(commands.serverSync.onCompleted, refreshWithoutRollOver);

  return { items, setItems, loaded: loadedDate === planDate };
}

type SetItems = Dispatch<SetStateAction<PlanItem[]>>;

/** Выполняет команду и подменяет задачу в списке результатом; ошибка команды даёт null. */
function usePlanItemUpdater(setItems: SetItems) {
  return useCallback(
    async (run: () => Promise<PlanItem>) => {
      const updated = await run().catch(() => null);
      if (updated) setItems((prev) => prev.map((item) => (item.id === updated.id ? updated : item)));
      return updated;
    },
    [setItems],
  );
}

/** Применяет результат команды и убирает задачу из списка, если она ушла на другой день. */
function usePlanItemKeepingDate(setItems: SetItems) {
  return useCallback(
    async (id: string, run: () => Promise<PlanItem>, staysVisible: (item: PlanItem) => boolean) => {
      const updated = await run().catch(() => null);
      if (updated) {
        setItems((prev) =>
          staysVisible(updated) ? prev.map((item) => (item.id === id ? updated : item)) : prev.filter((item) => item.id !== id),
        );
      }
      return updated;
    },
    [setItems],
  );
}

export function usePlanItems(planDate: string, autoRollOver = false) {
  const { items, setItems, loaded } = usePlanItemsData(planDate, autoRollOver);
  const update = usePlanItemUpdater(setItems);

  const updateKeepingDate = usePlanItemKeepingDate(setItems);

  const addItem = useCallback(
    async (title: string) => {
      const created = await commands.planItems.create(title, planDate).catch(() => null);
      if (created) setItems((prev) => [...prev, created]);
    },
    [planDate, setItems],
  );

  const deleteItem = useCallback(
    async (id: string) => {
      const previous = items;
      setItems((prev) => prev.filter((item) => item.id !== id));
      await commands.planItems.delete(id).catch(() => setItems(previous));
    },
    [items, setItems],
  );

  return {
    items,
    loaded,
    addItem,
    deleteItem,
    toggleDone: (id: string) => void update(() => commands.planItems.toggleDone(id)),
    cycleProgress: (id: string) => void update(() => commands.planItems.cycleProgress(id)),
    toggleDeferred: (id: string) => void update(() => commands.planItems.toggleDeferred(id)),
    toggleLongRunning: (id: string) =>
      updateKeepingDate(id, () => commands.planItems.toggleLongRunning(id), (item) => item.isLongRunning || item.planDate === planDate),
    moveToDate: (id: string, targetDate: string) =>
      void updateKeepingDate(id, () => commands.planItems.moveToDate(id, targetDate), (item) => item.planDate === planDate),
  };
}

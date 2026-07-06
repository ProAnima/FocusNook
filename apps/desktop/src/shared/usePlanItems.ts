import { useCallback, useEffect, useState } from "react";
import { commands, type PlanItem } from "./commands";

export function usePlanItems() {
  const [items, setItems] = useState<PlanItem[]>([]);
  const [loaded, setLoaded] = useState(false);

  useEffect(() => {
    commands.planItems
      .list()
      .then(setItems)
      .catch(() => {
        // Вне Tauri (browser-preview) список недоступен — остаёмся пустыми.
      })
      .finally(() => setLoaded(true));
  }, []);

  const addItem = useCallback(async (title: string) => {
    const created = await commands.planItems.create(title).catch(() => null);
    if (created) setItems((prev) => [...prev, created]);
  }, []);

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

  const deleteItem = useCallback(async (id: string) => {
    const previous = items;
    setItems((prev) => prev.filter((item) => item.id !== id));
    await commands.planItems.delete(id).catch(() => setItems(previous));
  }, [items]);

  return { items, loaded, addItem, toggleDone, cycleProgress, toggleDeferred, deleteItem };
}

import { useState } from "react";
import type { PlanItem } from "./commands";

/** Rotate through pending tasks. Keep successor IDs so completing the current task advances naturally. */
export function useZenQueue(items: PlanItem[]) {
  const [order, setOrder] = useState<string[]>([]);
  const pending = items.filter((item) => item.status !== "done");
  const current = order.map((id) => pending.find((item) => item.id === id)).find(Boolean) ?? pending[0];
  function next() {
    if (!current) return;
    const index = pending.findIndex((item) => item.id === current.id);
    setOrder([...pending.slice(index + 1), ...pending.slice(0, index + 1)].map((item) => item.id));
  }
  return { current, next };
}

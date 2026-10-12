import { useState } from "react";
import { ChevronRight } from "lucide-react";
import type { PlanItem } from "../shared/commands";
import { todayDateKey, addDays } from "../shared/dateKeys";
import { usePlanItems } from "../shared/usePlanItems";
import { useZenQueue } from "../shared/useZenQueue";
import { useZenWindowHeight } from "../shared/useZenWindowHeight";
import { useLocale } from "../shared/useLocale";
import { PlanItemRow, type PlanItemActions } from "./PlanItemList";
import { PlanItemRenameDialog } from "./PlanItemRenameDialog";
import { PlanItemDetailsDialog } from "./PlanItemDetailsDialog";

export function ZenView() {
  const { t } = useLocale();
  const today = todayDateKey();
  const plan = usePlanItems(today, true);
  const { current, next } = useZenQueue(plan.items);
  const viewRef = useZenWindowHeight(`${current?.id ?? "empty"}:${plan.loaded}`);
  const [details, setDetails] = useState<PlanItem | null>(null);
  const [renaming, setRenaming] = useState<PlanItem | null>(null);
  const actions: PlanItemActions = {
    onOpenDetails: setDetails, onRename: setRenaming,
    onToggleDone: plan.toggleDone, onCycleProgress: plan.cycleProgress,
    onToggleDeferred: plan.toggleDeferred, onToggleLongRunning: plan.toggleLongRunning,
    onMoveNextDay: (id) => plan.moveToDate(id, addDays(today, 1)), onDelete: plan.deleteItem,
  };
  const detailItem = plan.items.find((item) => item.id === details?.id);
  return (
    <section ref={viewRef} className="tab-view zen-view" aria-label={t("zen.enter")}>
      <div className="zen-current" aria-live="polite">
        {current ? <ul className="plan-list"><PlanItemRow item={current} actions={actions} /></ul> : plan.loaded && <p className="zen-finished">{t("zen.finished")}</p>}
      </div>
      {current && <button className="zen-next" type="button" onClick={next}>{t("zen.next")}<ChevronRight size={16} /></button>}
      {detailItem && <PlanItemDetailsDialog item={detailItem}
        onRename={() => { setRenaming(detailItem); setDetails(null); }}
        onToggleLongRunning={() => void plan.toggleLongRunning(detailItem.id)}
        onCycleProgress={() => plan.cycleProgress(detailItem.id)}
        onToggleDeferred={() => plan.toggleDeferred(detailItem.id)}
        onDelete={() => { void plan.deleteItem(detailItem.id); setDetails(null); }}
        onClose={() => setDetails(null)} />}
      {renaming && <PlanItemRenameDialog key={renaming.id} item={renaming} onSave={plan.rename} onClose={() => setRenaming(null)} />}
    </section>
  );
}

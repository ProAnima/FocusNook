import { useState, type FormEvent } from "react";
import { CalendarClock, Check, ListChecks, Plus } from "lucide-react";
import { usePlanItems } from "../shared/usePlanItems";
import type { PlanItem } from "../shared/commands";
import { EmptyState } from "./EmptyState";

const TODAY_LABEL = new Intl.DateTimeFormat("ru-RU", {
  weekday: "long",
  day: "numeric",
  month: "long",
}).format(new Date());

function PlanItemRow({
  item,
  onToggleDone,
}: {
  item: PlanItem;
  onToggleDone: (id: string) => void;
}) {
  return (
    <li className={`plan-item status-${item.status}`}>
      <button
        className="plan-checkbox"
        onClick={() => onToggleDone(item.id)}
        aria-label={
          item.status === "done" ? "Отметить невыполненным" : "Отметить выполненным"
        }
      >
        {item.status === "done" && <Check size={12} />}
      </button>
      <span className="plan-title">{item.title}</span>
      {item.status === "partial" && (
        <span className="plan-progress">{item.progressPercent}%</span>
      )}
      {item.status === "deferred" && (
        <CalendarClock size={13} className="plan-status-icon" />
      )}
    </li>
  );
}

export function DayView() {
  const { items, loaded, addItem, toggleDone } = usePlanItems();
  const [draft, setDraft] = useState("");
  const doneCount = items.filter((item) => item.status === "done").length;

  function handleSubmit(event: FormEvent) {
    event.preventDefault();
    const title = draft.trim();
    if (!title) return;
    setDraft("");
    void addItem(title);
  }

  return (
    <div className="tab-view">
      <div className="day-header">
        <span className="day-date">{TODAY_LABEL}</span>
        <span className="day-count">
          {doneCount}/{items.length}
        </span>
      </div>

      {loaded && items.length === 0 ? (
        <EmptyState icon={ListChecks} text="На сегодня пока ничего не запланировано" />
      ) : (
        <ul className="plan-list">
          {items.map((item) => (
            <PlanItemRow key={item.id} item={item} onToggleDone={toggleDone} />
          ))}
        </ul>
      )}

      <form className="quick-add" onSubmit={handleSubmit}>
        <Plus size={14} />
        <input
          placeholder="Добавить дело..."
          value={draft}
          onChange={(event) => setDraft(event.target.value)}
        />
      </form>
    </div>
  );
}

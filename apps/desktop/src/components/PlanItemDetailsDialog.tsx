import { useEffect, useRef } from "react";
import { X } from "lucide-react";
import type { PlanItem } from "../shared/commands";
import { useLocale } from "../shared/useLocale";

export function PlanItemDetailsDialog({ item, onClose }: { item: PlanItem; onClose: () => void }) {
  const closeRef = useRef<HTMLButtonElement>(null);
  const { t } = useLocale();

  useEffect(() => {
    closeRef.current?.focus();
    function handleKeyDown(event: KeyboardEvent) {
      if (event.key === "Escape") onClose();
    }
    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, [onClose]);

  return (
    <div
      className="plan-details-backdrop"
      role="presentation"
      onPointerDown={(event) => {
        if (event.currentTarget === event.target) onClose();
      }}
    >
      <section className="plan-details-dialog" role="dialog" aria-modal="true" aria-label={item.title}>
        <button
          ref={closeRef}
          className="icon-button plan-details-close"
          type="button"
          onClick={onClose}
          title={t("header.close")}
          aria-label={t("header.close")}
        >
          <X size={16} />
        </button>
        <p>{item.title}</p>
      </section>
    </div>
  );
}

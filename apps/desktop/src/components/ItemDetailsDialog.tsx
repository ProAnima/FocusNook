import { useEffect, useRef, type ReactNode } from "react";
import { X } from "lucide-react";
import { useLocale } from "../shared/useLocale";

// Focus, Escape and outside-click handling are kept in one accessible modal primitive.
// eslint-disable-next-line max-lines-per-function
export function ItemDetailsDialog({
  ariaLabel,
  children,
  onClose,
}: {
  ariaLabel: string;
  children: ReactNode;
  onClose: () => void;
}) {
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
      <section className="plan-details-dialog" role="dialog" aria-modal="true" aria-label={ariaLabel}>
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
        {children}
      </section>
    </div>
  );
}

import { useState, type FormEvent } from "react";
import type { PlanItem } from "../shared/commands";
import { useLocale } from "../shared/useLocale";
import { ItemDetailsDialog } from "./ItemDetailsDialog";

export function PlanItemRenameDialog({ item, onSave, onClose }: {
  item: PlanItem;
  onSave: (id: string, title: string) => Promise<PlanItem | null>;
  onClose: () => void;
}) {
  const { t } = useLocale();
  const [title, setTitle] = useState(item.title);
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState(false);
  async function submit(event: FormEvent) {
    event.preventDefault();
    if (!title.trim() || saving) return;
    setSaving(true);
    const updated = await onSave(item.id, title.trim());
    setSaving(false);
    if (updated) onClose();
    else setError(true);
  }
  return (
    <ItemDetailsDialog ariaLabel={t("day.rename")} onClose={onClose}>
      <form className="plan-rename-form" onSubmit={(event) => void submit(event)}>
        <label>{t("day.rename")}<input aria-label={t("day.rename")} value={title} onChange={(event) => setTitle(event.target.value)} disabled={saving} /></label>
        {error && <p role="alert">{t("day.renameError")}</p>}
        <div className="plan-details-actions">
          <button type="button" onClick={onClose}>{t("common.cancel")}</button>
          <button type="submit" disabled={saving || !title.trim()}>{t("common.save")}</button>
        </div>
      </form>
    </ItemDetailsDialog>
  );
}

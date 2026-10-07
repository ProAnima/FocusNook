import type { FormEvent } from "react";
import { Check, Folder, X } from "lucide-react";
import { useLocale } from "../../shared/useLocale";

export function FolderCreateForm({
  className,
  draft,
  iconSize,
  actionIconSize,
  onCancel,
  onChange,
  onSubmit,
}: {
  className: string;
  draft: string;
  iconSize: number;
  actionIconSize: number;
  onCancel: () => void;
  onChange: (value: string) => void;
  onSubmit: (event: FormEvent) => void;
}) {
  const { t } = useLocale();
  return (
    <form className={className} onSubmit={onSubmit}>
      <Folder size={iconSize} />
      <input
        autoFocus
        placeholder={t("notes.newFolder")}
        value={draft}
        onChange={(event) => onChange(event.target.value)}
        onKeyDown={(event) => {
          if (event.key === "Escape") onCancel();
        }}
      />
      <div className="note-folder-create-actions">
        <button className="icon-button" type="submit" disabled={!draft.trim()} title={t("common.save")} aria-label={t("common.save")}>
          <Check size={actionIconSize} />
        </button>
        <button className="icon-button" type="button" onClick={onCancel} title={t("common.cancel")} aria-label={t("common.cancel")}>
          <X size={actionIconSize} />
        </button>
      </div>
    </form>
  );
}

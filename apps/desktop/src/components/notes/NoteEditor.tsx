import { useState, type FormEvent } from "react";
import { Check, X } from "lucide-react";
import { useLocale } from "../../shared/useLocale";

export function NoteEditor({
  initialBody,
  onCancel,
  onSave,
}: {
  initialBody: string;
  onCancel: () => void;
  onSave: (body: string) => void;
}) {
  const [value, setValue] = useState(initialBody);
  const { t } = useLocale();
  const canSave = Boolean(value.trim()) && value.trim() !== initialBody;

  function save() {
    const body = value.trim();
    if (body) onSave(body);
  }

  function submit(event: FormEvent) {
    event.preventDefault();
    save();
  }

  return (
    <form className="note-editor" onSubmit={submit}>
      <textarea
        value={value}
        autoFocus
        onChange={(event) => setValue(event.target.value)}
        onKeyDown={(event) => {
          if (event.key === "Escape") onCancel();
          if ((event.ctrlKey || event.metaKey) && event.key === "Enter") save();
        }}
      />
      <div className="note-editor-actions">
        <button className="icon-button" type="submit" disabled={!canSave} title={t("common.save")} aria-label={t("common.save")}>
          <Check size={13} />
        </button>
        <button className="icon-button" type="button" onClick={onCancel} title={t("common.cancel")} aria-label={t("common.cancel")}>
          <X size={13} />
        </button>
      </div>
    </form>
  );
}

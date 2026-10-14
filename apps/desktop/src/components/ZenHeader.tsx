import { Maximize2 } from "lucide-react";
import { useLocale } from "../shared/useLocale";

export function ZenHeader({ onExit }: { onExit: () => void }) {
  const { t } = useLocale();
  return (
    <div className="zen-drag-zone" data-tauri-drag-region>
      <button className="icon-button" type="button" onClick={onExit} title={t("zen.exit")} aria-label={t("zen.exit")}>
        <Maximize2 size={13} />
      </button>
    </div>
  );
}

import { X } from "lucide-react";
import type { ReactNode } from "react";
import { commands } from "../shared/commands";
import { useLocale } from "../shared/useLocale";
import { WindowResizeHandles } from "./WindowResizeHandles";

export function AccountWindowChrome({ children }: { children: ReactNode }) {
  const { t } = useLocale();

  return (
    <div className="desktop-stage account-window-stage">
      <div className="overlay-shell account-window-shell">
        <header className="drag-zone account-window-drag" data-tauri-drag-region>
          <span className="brand" data-tauri-drag-region>FocusNook</span>
          <button
            className="icon-button"
            type="button"
            title={t("header.close")}
            aria-label={t("header.close")}
            onClick={() => void commands.overlay.close()}
          >
            <X size={14} />
          </button>
        </header>
        <main className="account-window-body">{children}</main>
        <WindowResizeHandles />
      </div>
    </div>
  );
}

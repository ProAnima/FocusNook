import { Pin, PinOff, Settings as SettingsIcon, X, type LucideIcon } from "lucide-react";
import { commands } from "../shared/commands";

function HeaderButton({
  icon: Icon,
  title,
  active,
  onClick,
}: {
  icon: LucideIcon;
  title: string;
  active?: boolean;
  onClick: () => void;
}) {
  return (
    <button
      className={`icon-button ${active ? "is-active" : ""}`}
      onClick={onClick}
      title={title}
    >
      <Icon size={14} />
    </button>
  );
}

interface OverlayHeaderProps {
  front: boolean;
  onToggleLayer: () => void;
  showSettings: boolean;
  onToggleSettings: () => void;
}

export function OverlayHeader({
  front,
  onToggleLayer,
  showSettings,
  onToggleSettings,
}: OverlayHeaderProps) {
  return (
    <header className="drag-zone" data-tauri-drag-region>
      <span className="brand">FocusNook</span>
      <div className="header-actions">
        <HeaderButton
          icon={front ? Pin : PinOff}
          active={front}
          onClick={onToggleLayer}
          title={front ? "Убрать с переднего плана" : "Показать поверх окон"}
        />
        <HeaderButton
          icon={SettingsIcon}
          active={showSettings}
          onClick={onToggleSettings}
          title="Настройки"
        />
        <HeaderButton
          icon={X}
          onClick={() => void commands.overlay.close()}
          title="Закрыть"
        />
      </div>
    </header>
  );
}

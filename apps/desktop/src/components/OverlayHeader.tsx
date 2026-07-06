import { Pin, PinOff, Settings as SettingsIcon, X, type LucideIcon } from "lucide-react";
import { commands, type Profile } from "../shared/commands";
import { useLocale } from "../shared/useLocale";
import { ProfileSwitcher } from "./ProfileSwitcher";

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
      aria-label={title}
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
  profiles: Profile[];
  activeProfileId: string | null;
  onSwitchProfile: (id: string) => void;
  onCreateProfile: (displayName: string) => void;
}

// Только для десктопа (см. App.tsx::DesktopShell) — на Android своя шапка
// (MobileShell), always-on-top и это меню там не нужны.
export function OverlayHeader({
  front,
  onToggleLayer,
  showSettings,
  onToggleSettings,
  profiles,
  activeProfileId,
  onSwitchProfile,
  onCreateProfile,
}: OverlayHeaderProps) {
  const { t } = useLocale();
  return (
    <header className="drag-zone" data-tauri-drag-region>
      <div className="header-left">
        <ProfileSwitcher
          profiles={profiles}
          activeProfileId={activeProfileId}
          onSwitch={onSwitchProfile}
          onCreate={onCreateProfile}
        />
        <span className="brand">FocusNook</span>
      </div>
      <div className="header-actions">
        <HeaderButton
          icon={front ? Pin : PinOff}
          active={front}
          onClick={onToggleLayer}
          title={front ? t("header.pinOff") : t("header.pinOn")}
        />
        <HeaderButton icon={SettingsIcon} active={showSettings} onClick={onToggleSettings} title={t("header.settings")} />
        <HeaderButton icon={X} onClick={() => void commands.overlay.close()} title={t("header.close")} />
      </div>
    </header>
  );
}

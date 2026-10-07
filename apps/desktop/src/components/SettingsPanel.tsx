import { X } from "lucide-react";
import { useTheme } from "../shared/useTheme";
import { useLocale } from "../shared/useLocale";
import type { ShortcutInfo } from "../shared/useLayerToggle";
import {
  AppearanceSection,
  AutostartSection,
  DiagnosticsSection,
  LanguageSection,
  MicrophoneSection,
  NoteFoldersSection,
  ShortcutSection,
} from "./settings/PreferenceSections";
import { SyncSection } from "./settings/SyncSection";

export function SettingsPanel({
  shortcutInfo,
  onClose,
  isDesktop,
}: {
  shortcutInfo: ShortcutInfo | null;
  onClose: () => void;
  isDesktop: boolean;
}) {
  const { mode, setMode } = useTheme();
  const { t } = useLocale();

  return (
    <div className="settings-panel">
      {isDesktop && (
        <div className="settings-header">
          <span>{t("settings.title")}</span>
          <button className="icon-button" onClick={onClose} title={t("header.close")} aria-label={t("header.close")}>
            <X size={14} />
          </button>
        </div>
      )}

      <AppearanceSection mode={mode} setMode={setMode} />
      <LanguageSection />
      <NoteFoldersSection />
      <MicrophoneSection />
      {/* "Запускать вместе с Windows" не имеет смысла на телефоне. */}
      {isDesktop && <AutostartSection />}
      {shortcutInfo && <ShortcutSection info={shortcutInfo} />}
      <SyncSection />
      <DiagnosticsSection />
    </div>
  );
}

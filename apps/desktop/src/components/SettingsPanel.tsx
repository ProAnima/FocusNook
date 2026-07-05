import { useEffect, useState } from "react";
import { Monitor, Moon, Sun, X } from "lucide-react";
import { useTheme, type ThemeMode } from "../shared/useTheme";
import { commands } from "../shared/commands";
import type { ShortcutInfo } from "../shared/useLayerToggle";

const THEME_OPTIONS: { mode: ThemeMode; label: string; icon: typeof Sun }[] = [
  { mode: "system", label: "Системная", icon: Monitor },
  { mode: "light", label: "Светлая", icon: Sun },
  { mode: "dark", label: "Тёмная", icon: Moon },
];

function ThemeSection({
  mode,
  setMode,
}: {
  mode: ThemeMode;
  setMode: (mode: ThemeMode) => void;
}) {
  return (
    <div className="settings-group">
      <span className="settings-group-label">Тема</span>
      <div className="theme-options">
        {THEME_OPTIONS.map(({ mode: optionMode, label, icon: Icon }) => (
          <button
            key={optionMode}
            className={`theme-option ${mode === optionMode ? "is-active" : ""}`}
            onClick={() => setMode(optionMode)}
          >
            <Icon size={15} />
            <span>{label}</span>
          </button>
        ))}
      </div>
    </div>
  );
}

function AutostartSection() {
  const [autostart, setAutostart] = useState(false);

  useEffect(() => {
    commands.settings
      .getAutostart()
      .then(setAutostart)
      .catch(() => {
        // Вне Tauri автостарт недоступен — оставляем выключенным.
      });
  }, []);

  async function toggle() {
    const next = !autostart;
    setAutostart(next);
    try {
      await commands.settings.setAutostart(next);
    } catch {
      setAutostart(!next);
    }
  }

  return (
    <div className="settings-group">
      <span className="settings-group-label">Автозапуск</span>
      <button className="toggle-row" onClick={toggle}>
        <span>Запускать вместе с Windows</span>
        <span className={`toggle-switch ${autostart ? "is-on" : ""}`} />
      </button>
    </div>
  );
}

function ShortcutSection({ info }: { info: ShortcutInfo }) {
  return (
    <div className="settings-group">
      <span className="settings-group-label">Хоткей "поверх окон"</span>
      <p className="settings-hint">
        {info.shortcut.replace(/\+/g, " + ").toUpperCase()}
        {info.isFallback && " — запасной, основной был занят другой программой"}
      </p>
    </div>
  );
}

export function SettingsPanel({
  shortcutInfo,
  onClose,
}: {
  shortcutInfo: ShortcutInfo | null;
  onClose: () => void;
}) {
  const { mode, setMode } = useTheme();

  return (
    <div className="settings-panel">
      <div className="settings-header">
        <span>Настройки</span>
        <button className="icon-button" onClick={onClose} title="Закрыть">
          <X size={14} />
        </button>
      </div>

      <ThemeSection mode={mode} setMode={setMode} />
      <AutostartSection />
      {shortcutInfo && <ShortcutSection info={shortcutInfo} />}
    </div>
  );
}

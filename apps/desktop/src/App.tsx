import { useEffect, useState } from "react";
import {
  BellRing,
  CalendarDays,
  NotebookPen,
  Settings as SettingsIcon,
} from "lucide-react";
import { commands, isAlertWindow, type FolderRailSide } from "./shared/commands";
import { ThemeProvider } from "./shared/theme";
import { LocaleProvider } from "./shared/locale";
import { useLayerToggle, type ShortcutInfo } from "./shared/useLayerToggle";
import { useLiveBackgroundPointer } from "./shared/useLiveBackgroundPointer";
import { useLocale } from "./shared/useLocale";
import { useProfiles } from "./shared/useProfiles";
import { useTheme } from "./shared/useTheme";
import type { ResolvedTheme } from "./shared/theme-context";
import { SettingsPanel } from "./components/SettingsPanel";
import { DayView } from "./components/DayView";
import { NotesView } from "./components/NotesView";
import { RemindersView } from "./components/RemindersView";
import { ReminderAlert } from "./components/ReminderAlert";
import { LiveBackground } from "./components/LiveBackground";
import { OverlayHeader } from "./components/OverlayHeader";
import { TabBar, type TabDefinition } from "./components/TabBar";
import "./App.css";

type TabKey = "day" | "notes" | "reminders" | "settings";

function useMainTabs(): readonly TabDefinition<TabKey>[] {
  const { t } = useLocale();
  return [
    { key: "day", label: t("nav.day"), icon: CalendarDays },
    { key: "notes", label: t("nav.notes"), icon: NotebookPen },
    { key: "reminders", label: t("nav.reminders"), icon: BellRing },
  ];
}

function useMobileTabs(): readonly TabDefinition<TabKey>[] {
  const { t } = useLocale();
  const mainTabs = useMainTabs();
  return [...mainTabs, { key: "settings", label: t("nav.settings"), icon: SettingsIcon }];
}

function TabContent({
  tab,
  shortcutInfo,
  onCloseSettings,
  isDesktop,
}: {
  tab: TabKey;
  shortcutInfo: ShortcutInfo | null;
  onCloseSettings: () => void;
  isDesktop: boolean;
}) {
  switch (tab) {
    case "day":
      return <DayView />;
    case "notes":
      return <NotesView />;
    case "reminders":
      return <RemindersView />;
    case "settings":
      return (
        <SettingsPanel shortcutInfo={shortcutInfo} onClose={onCloseSettings} isDesktop={isDesktop} />
      );
  }
}

interface DesktopShellProps {
  front: boolean;
  toggleLayer: () => void;
  shortcutInfo: ShortcutInfo | null;
  theme: ResolvedTheme;
  folderRailSide: FolderRailSide;
}

// Desktop: тихий угловой оверлей (раздел 12 ТЗ) — настройки нарочно не
// четвёртая вкладка, а ненавязчивый экран за иконкой в шапке.
function DesktopShell({ front, toggleLayer, shortcutInfo, theme, folderRailSide }: DesktopShellProps) {
  const [activeTab, setActiveTab] = useState<TabKey>("day");
  const [showSettings, setShowSettings] = useState(false);
  const { profiles, activeProfileId, createProfile, switchProfile } = useProfiles();
  const mainTabs = useMainTabs();

  return (
    <div className="desktop-stage" data-folder-rail-side={folderRailSide}>
      <div className="overlay-shell">
        <LiveBackground theme={theme} />
        <OverlayHeader
          front={front}
          onToggleLayer={toggleLayer}
          showSettings={showSettings}
          onToggleSettings={() => setShowSettings((value) => !value)}
          profiles={profiles}
          activeProfileId={activeProfileId}
          onSwitchProfile={switchProfile}
          onCreateProfile={createProfile}
        />

        {!showSettings && <TabBar tabs={mainTabs} active={activeTab} onSelect={setActiveTab} />}

        <main className="body">
          {showSettings ? (
            <SettingsPanel
              shortcutInfo={shortcutInfo}
              onClose={() => setShowSettings(false)}
              isDesktop
            />
          ) : (
            <TabContent
              key={activeProfileId}
              tab={activeTab}
              shortcutInfo={shortcutInfo}
              onCloseSettings={() => {}}
              isDesktop
            />
          )}
        </main>
      </div>
    </div>
  );
}

function useFolderRailSide(isDesktop: boolean): FolderRailSide {
  const [side, setSide] = useState<FolderRailSide>("left");

  useEffect(() => {
    if (!isDesktop) return;

    let cancelled = false;
    let unlisten: (() => void) | null = null;
    let intervalId: number | null = null;

    function refreshSide() {
      void commands.overlay.getFolderRailSide().then((nextSide) => {
        if (!cancelled) setSide(nextSide);
      });
    }

    refreshSide();
    intervalId = window.setInterval(refreshSide, 350);
    commands.overlay
      .onFolderRailSideChanged((nextSide) => {
        if (!cancelled) setSide(nextSide);
      })
      .then((dispose) => {
        unlisten = dispose;
      })
      .catch(() => {});

    return () => {
      cancelled = true;
      if (intervalId !== null) window.clearInterval(intervalId);
      unlisten?.();
    };
  }, [isDesktop]);

  return side;
}

// Mobile: полноэкранное приложение, а не растянутый десктопный виджет —
// нижняя навигация (настройки — обычный, легко доступный большим пальцем
// пункт, а не спрятанная иконка в верхнем углу) и контекстный заголовок сверху
// вместо статичного бренда.
function MobileShell({
  shortcutInfo,
  theme,
}: {
  shortcutInfo: ShortcutInfo | null;
  theme: ResolvedTheme;
}) {
  const [activeTab, setActiveTab] = useState<TabKey>("day");
  const mobileTabs = useMobileTabs();
  const activeLabel = mobileTabs.find((tab) => tab.key === activeTab)?.label ?? "";

  return (
    <div className="overlay-shell mobile-shell">
      <LiveBackground theme={theme} />
      <header className="mobile-topbar">
        <span className="mobile-topbar-title">{activeLabel}</span>
      </header>

      <main className="body">
        <TabContent
          tab={activeTab}
          shortcutInfo={shortcutInfo}
          onCloseSettings={() => setActiveTab("day")}
          isDesktop={false}
        />
      </main>

      <TabBar
        tabs={mobileTabs}
        active={activeTab}
        onSelect={setActiveTab}
        className="bottom-nav"
        iconSize={20}
      />
    </div>
  );
}

function Shell() {
  const { front, toggleLayer, shortcutInfo, isDesktop } = useLayerToggle();
  const { effective } = useTheme();
  const folderRailSide = useFolderRailSide(isDesktop);
  useLiveBackgroundPointer(effective);
  return isDesktop ? (
    <DesktopShell
      front={front}
      toggleLayer={toggleLayer}
      shortcutInfo={shortcutInfo}
      theme={effective}
      folderRailSide={folderRailSide}
    />
  ) : (
    <MobileShell shortcutInfo={shortcutInfo} theme={effective} />
  );
}

export default function App() {
  return (
    <LocaleProvider>
      <ThemeProvider>{isAlertWindow() ? <ReminderAlert /> : <Shell />}</ThemeProvider>
    </LocaleProvider>
  );
}

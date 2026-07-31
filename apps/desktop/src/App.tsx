import { useEffect, useState } from "react";
import {
  BellRing,
  CalendarDays,
  NotebookPen,
  LogOut,
  Settings as SettingsIcon,
} from "lucide-react";
import { commands, isAlertWindow, type FolderRailSide } from "./shared/commands";
import { ThemeProvider } from "./shared/theme";
import { LocaleProvider } from "./shared/locale";
import { useLayerToggle, type ShortcutInfo } from "./shared/useLayerToggle";
import { useDesktopCursorPassthrough } from "./shared/useDesktopCursorPassthrough";
import { useLiveBackgroundPointer } from "./shared/useLiveBackgroundPointer";
import { useServerSyncWakeup } from "./shared/useServerSyncWakeup";
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
import { WindowResizeHandles } from "./components/WindowResizeHandles";
import { AccountGate } from "./components/AccountGate";
import { AccountWindowChrome } from "./components/AccountWindowChrome";
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
      return <NotesView isDesktop={isDesktop} />;
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
  accounts: ReturnType<typeof useProfiles>;
}

// Desktop: тихий угловой оверлей (раздел 12 ТЗ) — настройки нарочно не
// четвёртая вкладка, а ненавязчивый экран за иконкой в шапке.
function DesktopShell({ front, toggleLayer, shortcutInfo, theme, folderRailSide, accounts }: DesktopShellProps) {
  const [activeTab, setActiveTab] = useState<TabKey>("day");
  const [showSettings, setShowSettings] = useState(false);
  const { activeProfileId, activeProfile, logout } = accounts;
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
          account={activeProfile!}
          onLogout={() => void logout()}
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
        <WindowResizeHandles />
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
  accounts,
}: {
  shortcutInfo: ShortcutInfo | null;
  theme: ResolvedTheme;
  accounts: ReturnType<typeof useProfiles>;
}) {
  const [activeTab, setActiveTab] = useState<TabKey>("day");
  const { t } = useLocale();
  const mobileTabs = useMobileTabs();
  const activeLabel = mobileTabs.find((tab) => tab.key === activeTab)?.label ?? "";

  return (
    <div className="overlay-shell mobile-shell">
      <LiveBackground theme={theme} />
      <header className="mobile-topbar">
        <span className="mobile-topbar-title">{activeLabel}</span>
        <button className="icon-button" onClick={() => void accounts.logout()} title={t("account.logout")} aria-label={t("account.logout")}>
          <LogOut size={16} />
        </button>
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
  const accounts = useProfiles();
  useDesktopCursorPassthrough(isDesktop);
  useLiveBackgroundPointer(effective);
  useServerSyncWakeup();
  if (accounts.loading) return <div className="account-gate" />;
  const setupRequired = !accounts.activeProfile?.accountConfigured;
  if (setupRequired || accounts.sessionLocked) {
    const gate = (
      <AccountGate
        accounts={accounts.profiles}
        activeAccount={accounts.activeProfile}
        setupRequired={setupRequired}
        onConfigure={accounts.configureAccount}
        onCreate={accounts.createAccount}
        onSignIn={accounts.switchAccount}
      />
    );
    return isDesktop ? <AccountWindowChrome>{gate}</AccountWindowChrome> : gate;
  }
  return isDesktop ? (
    <DesktopShell
      front={front}
      toggleLayer={toggleLayer}
      shortcutInfo={shortcutInfo}
      theme={effective}
      folderRailSide={folderRailSide}
      accounts={accounts}
    />
  ) : (
    <MobileShell shortcutInfo={shortcutInfo} theme={effective} accounts={accounts} />
  );
}

export default function App() {
  return (
    <LocaleProvider>
      <ThemeProvider>{isAlertWindow() ? <ReminderAlert /> : <Shell />}</ThemeProvider>
    </LocaleProvider>
  );
}

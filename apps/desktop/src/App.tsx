import { useState } from "react";
import { BellRing, CalendarDays, NotebookPen } from "lucide-react";
import { isAlertWindow } from "./shared/commands";
import { ThemeProvider } from "./shared/theme";
import { useLayerToggle } from "./shared/useLayerToggle";
import { SettingsPanel } from "./components/SettingsPanel";
import { DayView } from "./components/DayView";
import { NotesView } from "./components/NotesView";
import { RemindersView } from "./components/RemindersView";
import { ReminderAlert } from "./components/ReminderAlert";
import { OverlayHeader } from "./components/OverlayHeader";
import { TabBar } from "./components/TabBar";
import "./App.css";

const TABS = [
  { key: "day", label: "День", icon: CalendarDays },
  { key: "notes", label: "Заметки", icon: NotebookPen },
  { key: "reminders", label: "Напоминания", icon: BellRing },
] as const;

type TabKey = (typeof TABS)[number]["key"];

function Shell() {
  const { front, toggleLayer, shortcutInfo } = useLayerToggle();
  const [activeTab, setActiveTab] = useState<TabKey>("day");
  const [showSettings, setShowSettings] = useState(false);

  return (
    <div className="overlay-shell">
      <OverlayHeader
        front={front}
        onToggleLayer={toggleLayer}
        showSettings={showSettings}
        onToggleSettings={() => setShowSettings((value) => !value)}
      />

      {!showSettings && (
        <TabBar tabs={TABS} active={activeTab} onSelect={setActiveTab} />
      )}

      <main className="body">
        {showSettings ? (
          <SettingsPanel
            shortcutInfo={shortcutInfo}
            onClose={() => setShowSettings(false)}
          />
        ) : activeTab === "day" ? (
          <DayView />
        ) : activeTab === "notes" ? (
          <NotesView />
        ) : (
          <RemindersView />
        )}
      </main>
    </div>
  );
}

export default function App() {
  return (
    <ThemeProvider>{isAlertWindow() ? <ReminderAlert /> : <Shell />}</ThemeProvider>
  );
}

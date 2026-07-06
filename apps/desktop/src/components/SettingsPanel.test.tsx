import { describe, expect, it, vi } from "vitest";
import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { SettingsPanel } from "./SettingsPanel";
import { LocaleProvider } from "../shared/locale";

const {
  getAutostart,
  setAutostart,
  setMode,
  getLocale,
  setLocale,
  getMicrophoneDeviceId,
  setMicrophoneDeviceId,
  exportDiagnostics,
  syncStatus,
  syncStart,
  syncDisconnect,
  serverSyncStatus,
  serverSyncConnect,
  serverSyncDisconnect,
} = vi.hoisted(() => ({
  getAutostart: vi.fn().mockResolvedValue(false),
  setAutostart: vi.fn().mockResolvedValue(undefined),
  setMode: vi.fn(),
  getLocale: vi.fn().mockResolvedValue(null),
  setLocale: vi.fn().mockResolvedValue(undefined),
  getMicrophoneDeviceId: vi.fn().mockResolvedValue(null),
  setMicrophoneDeviceId: vi.fn().mockResolvedValue(undefined),
  exportDiagnostics: vi.fn(),
  syncStatus: vi.fn().mockResolvedValue({ connected: false }),
  syncStart: vi.fn().mockResolvedValue(undefined),
  syncDisconnect: vi.fn().mockResolvedValue(undefined),
  serverSyncStatus: vi.fn().mockResolvedValue({ connected: false, endpoint: null }),
  serverSyncConnect: vi.fn().mockResolvedValue({ connected: true, endpoint: "https://sync.example.com" }),
  serverSyncDisconnect: vi.fn().mockResolvedValue(undefined),
}));

vi.mock("../shared/commands", () => ({
  commands: {
    settings: { getAutostart, setAutostart, getLocale, setLocale, getMicrophoneDeviceId, setMicrophoneDeviceId },
    diagnostics: { export: exportDiagnostics },
    sync: { status: syncStatus, start: syncStart, disconnect: syncDisconnect },
    serverSync: { status: serverSyncStatus, connect: serverSyncConnect, disconnect: serverSyncDisconnect },
  },
}));

vi.mock("../shared/useTheme", () => ({
  useTheme: () => ({ mode: "system", effective: "dark", setMode }),
}));

describe("SettingsPanel", () => {
  it("turns autostart on when the toggle row is clicked", async () => {
    const user = userEvent.setup();
    render(<SettingsPanel shortcutInfo={null} onClose={() => {}} isDesktop />);

    await user.click(await screen.findByText("Запускать вместе с Windows"));

    expect(setAutostart).toHaveBeenCalledWith(true);
  });

  it("switches to a live theme when its swatch is clicked", async () => {
    const user = userEvent.setup();
    render(<SettingsPanel shortcutInfo={null} onClose={() => {}} isDesktop />);

    await user.click(await screen.findByRole("button", { name: /Системная/ }));
    await user.click(await screen.findByText("Закат"));

    expect(setMode).toHaveBeenCalledWith("sunset");
  });

  it("keeps theme variants collapsed until the section is opened", async () => {
    const user = userEvent.setup();
    const { container } = render(<SettingsPanel shortcutInfo={null} onClose={() => {}} isDesktop />);

    expect(container.querySelectorAll(".theme-row")).toHaveLength(0);
    await user.click(await screen.findByRole("button", { name: /Системная/ }));
    await screen.findByText("Призма");

    expect(container.querySelectorAll(".theme-row")).toHaveLength(10);
  });

  it("hides the autostart section on the mobile shell", async () => {
    render(<SettingsPanel shortcutInfo={null} onClose={() => {}} isDesktop={false} />);

    await screen.findByText("Настройки");
    expect(screen.queryByText("Запускать вместе с Windows")).not.toBeInTheDocument();
  });

  it("shows the active shortcut and flags a fallback", () => {
    render(
      <SettingsPanel
        shortcutInfo={{ shortcut: "ctrl+alt+space", isFallback: true }}
        onClose={() => {}}
        isDesktop
      />,
    );

    expect(screen.getByText(/CTRL \+ ALT \+ SPACE/)).toBeInTheDocument();
    expect(screen.getByText(/запасной/)).toBeInTheDocument();
  });

  it("switches the UI language when a language option is clicked", async () => {
    const user = userEvent.setup();
    render(
      <LocaleProvider>
        <SettingsPanel shortcutInfo={null} onClose={() => {}} isDesktop />
      </LocaleProvider>,
    );

    await screen.findByText("Настройки");
    await user.click(screen.getByRole("button", { name: "Язык" }));
    await user.click(screen.getByRole("option", { name: "English" }));

    expect(setLocale).toHaveBeenCalledWith("en");
    expect(await screen.findByText("Settings")).toBeInTheDocument();
  });

  it("offers ten popular UI languages", async () => {
    const user = userEvent.setup();
    render(<SettingsPanel shortcutInfo={null} onClose={() => {}} isDesktop />);

    await user.click(screen.getByRole("button", { name: "Язык" }));

    expect(screen.getAllByRole("option")).toHaveLength(10);
  });

  it("shows the saved path after exporting diagnostics", async () => {
    exportDiagnostics.mockResolvedValue("C:\\Users\\test\\AppData\\diagnostics-2026-07-05.json");
    const user = userEvent.setup();
    render(<SettingsPanel shortcutInfo={null} onClose={() => {}} isDesktop />);

    await user.click(screen.getByText("Экспортировать диагностику"));

    expect(await screen.findByText(/diagnostics-2026-07-05\.json/)).toBeInTheDocument();
  });

  it("shows an error when the diagnostics export fails", async () => {
    exportDiagnostics.mockRejectedValue(new Error("disk full"));
    const user = userEvent.setup();
    render(<SettingsPanel shortcutInfo={null} onClose={() => {}} isDesktop />);

    await user.click(screen.getByText("Экспортировать диагностику"));

    expect(await screen.findByText("Не удалось экспортировать диагностику")).toBeInTheDocument();
  });

  it("shows not-connected status for both sync providers by default", async () => {
    render(<SettingsPanel shortcutInfo={null} onClose={() => {}} isDesktop />);

    expect(await screen.findAllByText("Не подключено")).toHaveLength(2);
  });

  it("starts the auth flow when Connect is clicked for a disconnected provider", async () => {
    const user = userEvent.setup();
    render(<SettingsPanel shortcutInfo={null} onClose={() => {}} isDesktop />);

    await screen.findAllByText("Не подключено");
    const [googleConnect] = await screen.findAllByText("Подключить");
    await user.click(googleConnect);

    expect(syncStart).toHaveBeenCalledWith("google_drive");
  });

  it("shows an error when starting auth fails", async () => {
    syncStart.mockRejectedValueOnce(new Error("provider not configured"));
    const user = userEvent.setup();
    render(<SettingsPanel shortcutInfo={null} onClose={() => {}} isDesktop />);

    const [googleConnect] = await screen.findAllByText("Подключить");
    await user.click(googleConnect);

    expect(await screen.findByText("Не удалось подключить — проверьте настройку провайдера")).toBeInTheDocument();
  });

  it("disconnects an already-connected provider", async () => {
    syncStatus.mockImplementation((provider: string) =>
      Promise.resolve({ connected: provider === "yandex_disk" }),
    );
    const user = userEvent.setup();
    render(<SettingsPanel shortcutInfo={null} onClose={() => {}} isDesktop />);

    const disconnectButton = await screen.findByText("Отключить");
    await user.click(disconnectButton);

    expect(syncDisconnect).toHaveBeenCalledWith("yandex_disk");
  });

  it("saves the custom server sync connection", async () => {
    const user = userEvent.setup();
    render(<SettingsPanel shortcutInfo={null} onClose={() => {}} isDesktop />);

    await user.type(screen.getByLabelText("https://sync.example.com"), "https://sync.focusnook.test");
    await user.type(screen.getByLabelText("Токен доступа"), "server-token");
    const serverCard = screen.getByText("ProAnima VDS").closest(".sync-provider-row");
    const connectButton = serverCard?.querySelector("button");

    expect(connectButton).toBeTruthy();
    await user.click(connectButton as HTMLButtonElement);

    expect(serverSyncConnect).toHaveBeenCalledWith("https://sync.focusnook.test", "server-token");
  });
});

import { describe, expect, it, vi } from "vitest";
import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { SettingsPanel } from "./SettingsPanel";

const { getAutostart, setAutostart, setMode } = vi.hoisted(() => ({
  getAutostart: vi.fn().mockResolvedValue(false),
  setAutostart: vi.fn().mockResolvedValue(undefined),
  setMode: vi.fn(),
}));

vi.mock("../shared/commands", () => ({
  commands: { settings: { getAutostart, setAutostart } },
}));

vi.mock("../shared/useTheme", () => ({
  useTheme: () => ({ mode: "system", effective: "dark", setMode }),
}));

describe("SettingsPanel", () => {
  it("turns autostart on when the toggle row is clicked", async () => {
    const user = userEvent.setup();
    render(<SettingsPanel shortcutInfo={null} onClose={() => {}} />);

    await user.click(await screen.findByText("Запускать вместе с Windows"));

    expect(setAutostart).toHaveBeenCalledWith(true);
  });

  it("shows the active shortcut and flags a fallback", () => {
    render(
      <SettingsPanel
        shortcutInfo={{ shortcut: "ctrl+alt+space", isFallback: true }}
        onClose={() => {}}
      />,
    );

    expect(screen.getByText(/CTRL \+ ALT \+ SPACE/)).toBeInTheDocument();
    expect(screen.getByText(/запасной/)).toBeInTheDocument();
  });
});

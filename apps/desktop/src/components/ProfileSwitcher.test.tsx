import { describe, expect, it, vi } from "vitest";
import { fireEvent, render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { ProfileSwitcher } from "./ProfileSwitcher";

const PROFILES = [
  { id: "1", displayName: "Личный", avatarColor: "#f2b463" },
  { id: "2", displayName: "Рабочий", avatarColor: "#7cb9e8" },
];

describe("ProfileSwitcher", () => {
  it("shows the active profile's initial on the avatar", () => {
    render(
      <ProfileSwitcher
        profiles={PROFILES}
        activeProfileId="2"
        onSwitch={vi.fn()}
        onCreate={vi.fn()}
      />,
    );

    expect(screen.getByTitle("Рабочий")).toHaveTextContent("Р");
  });

  it("opens the dropdown and lists all profiles on click", async () => {
    const user = userEvent.setup();
    render(
      <ProfileSwitcher
        profiles={PROFILES}
        activeProfileId="1"
        onSwitch={vi.fn()}
        onCreate={vi.fn()}
      />,
    );

    expect(screen.queryByText("Рабочий")).not.toBeInTheDocument();
    await user.click(screen.getByTitle("Личный"));

    expect(screen.getByText("Личный")).toBeInTheDocument();
    expect(screen.getByText("Рабочий")).toBeInTheDocument();
  });

  it("calls onSwitch when another profile is selected", async () => {
    const onSwitch = vi.fn();
    const user = userEvent.setup();
    render(
      <ProfileSwitcher
        profiles={PROFILES}
        activeProfileId="1"
        onSwitch={onSwitch}
        onCreate={vi.fn()}
      />,
    );

    await user.click(screen.getByTitle("Личный"));
    await user.click(screen.getByText("Рабочий"));

    expect(onSwitch).toHaveBeenCalledWith("2");
  });

  it("calls onCreate with the typed name from the new-profile form", async () => {
    const onCreate = vi.fn();
    const user = userEvent.setup();
    render(
      <ProfileSwitcher
        profiles={PROFILES}
        activeProfileId="1"
        onSwitch={vi.fn()}
        onCreate={onCreate}
      />,
    );

    await user.click(screen.getByTitle("Личный"));
    await user.type(screen.getByPlaceholderText("Новый профиль..."), "Учёба{Enter}");

    expect(onCreate).toHaveBeenCalledWith("Учёба");
  });

  it("closes the dropdown when clicking outside", async () => {
    const user = userEvent.setup();
    render(
      <div>
        <ProfileSwitcher
          profiles={PROFILES}
          activeProfileId="1"
          onSwitch={vi.fn()}
          onCreate={vi.fn()}
        />
        <div data-testid="outside">Снаружи</div>
      </div>,
    );

    await user.click(screen.getByTitle("Личный"));
    expect(screen.getByText("Рабочий")).toBeInTheDocument();

    fireEvent.mouseDown(screen.getByTestId("outside"));
    expect(screen.queryByText("Рабочий")).not.toBeInTheDocument();
  });

  it("closes the dropdown on Escape", async () => {
    const user = userEvent.setup();
    render(
      <ProfileSwitcher
        profiles={PROFILES}
        activeProfileId="1"
        onSwitch={vi.fn()}
        onCreate={vi.fn()}
      />,
    );

    await user.click(screen.getByTitle("Личный"));
    expect(screen.getByText("Рабочий")).toBeInTheDocument();

    await user.keyboard("{Escape}");
    expect(screen.queryByText("Рабочий")).not.toBeInTheDocument();
  });

  it("exposes an accessible name and expanded state on the avatar button", async () => {
    const user = userEvent.setup();
    render(
      <ProfileSwitcher
        profiles={PROFILES}
        activeProfileId="1"
        onSwitch={vi.fn()}
        onCreate={vi.fn()}
      />,
    );

    const avatar = screen.getByRole("button", { name: "Личный" });
    expect(avatar).toHaveAttribute("aria-expanded", "false");

    await user.click(avatar);
    expect(avatar).toHaveAttribute("aria-expanded", "true");
  });
});

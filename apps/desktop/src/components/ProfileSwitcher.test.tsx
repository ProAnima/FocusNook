import { describe, expect, it, vi } from "vitest";
import { fireEvent, render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { ProfileSwitcher } from "./ProfileSwitcher";

const ACCOUNT = {
  id: "1",
  displayName: "Личный",
  avatarColor: "#f2b463",
  email: "me@example.test",
  accountConfigured: true,
  syncEnabled: false,
};

describe("ProfileSwitcher", () => {
  it("shows the account identity and logs out", async () => {
    const onLogout = vi.fn();
    const user = userEvent.setup();
    render(<ProfileSwitcher account={ACCOUNT} onLogout={onLogout} />);

    const avatar = screen.getByRole("button", { name: "Личный" });
    expect(avatar).toHaveTextContent("Л");
    expect(avatar).toHaveAttribute("aria-expanded", "false");
    await user.click(avatar);

    expect(avatar).toHaveAttribute("aria-expanded", "true");
    expect(screen.getByText("me@example.test")).toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: /выйти/i }));
    expect(onLogout).toHaveBeenCalledOnce();
  });

  it("closes the menu when clicking outside", async () => {
    const user = userEvent.setup();
    render(<div><ProfileSwitcher account={ACCOUNT} onLogout={vi.fn()} /><div data-testid="outside" /></div>);
    await user.click(screen.getByRole("button", { name: "Личный" }));
    expect(screen.getByText("me@example.test")).toBeInTheDocument();
    fireEvent.mouseDown(screen.getByTestId("outside"));
    expect(screen.queryByText("me@example.test")).not.toBeInTheDocument();
  });
});

import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import { AccountGate } from "./AccountGate";

const ACCOUNT = {
  id: "one",
  displayName: "Ян",
  avatarColor: "#f2b463",
  email: "i@example.test",
  accountConfigured: true,
  syncEnabled: false,
};

describe("AccountGate", () => {
  it("requires the account password when signing in", async () => {
    const onSignIn = vi.fn().mockResolvedValue(undefined);
    const user = userEvent.setup();
    render(
      <AccountGate
        accounts={[ACCOUNT]}
        activeAccount={ACCOUNT}
        setupRequired={false}
        onConfigure={vi.fn()}
        onCreate={vi.fn()}
        onSignIn={onSignIn}
      />,
    );

    await user.type(screen.getByLabelText("Пароль"), "StrongPass123");
    await user.click(screen.getByRole("button", { name: "Войти" }));
    expect(onSignIn).toHaveBeenCalledWith("one", "StrongPass123");
  });

  it("does not create an account when password confirmation differs", async () => {
    const onConfigure = vi.fn().mockResolvedValue(undefined);
    const user = userEvent.setup();
    render(
      <AccountGate
        accounts={[{ ...ACCOUNT, email: null, accountConfigured: false }]}
        activeAccount={{ ...ACCOUNT, email: null, accountConfigured: false }}
        setupRequired
        onConfigure={onConfigure}
        onCreate={vi.fn()}
        onSignIn={vi.fn()}
      />,
    );

    await user.clear(screen.getByLabelText("Имя"));
    await user.type(screen.getByLabelText("Имя"), "Ян");
    await user.type(screen.getByLabelText("Email"), "i@example.test");
    await user.type(screen.getByLabelText("Пароль"), "StrongPass123");
    await user.type(screen.getByLabelText(/Повторите пароль/), "Different123");
    await user.click(screen.getByRole("button", { name: "Создать аккаунт" }));
    expect(screen.getByRole("alert")).toHaveTextContent("Пароли не совпадают");
    expect(onConfigure).not.toHaveBeenCalled();
  });
});

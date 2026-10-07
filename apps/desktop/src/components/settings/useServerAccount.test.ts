import { beforeEach, describe, expect, it, vi } from "vitest";
import { act, renderHook, waitFor } from "@testing-library/react";
import { useServerAccount } from "./useServerAccount";

const { status, login, register, disconnect, deleteAccount } = vi.hoisted(() => ({
  status: vi.fn(),
  login: vi.fn(),
  register: vi.fn(),
  disconnect: vi.fn(),
  deleteAccount: vi.fn(),
}));

vi.mock("../../shared/commands", () => ({
  commands: { serverSync: { status, login, register, disconnect, deleteAccount } },
}));

const connected = {
  available: true,
  connected: true,
  accountEmail: "user@example.com",
  displayName: "User",
  mediaReady: true,
  endpoint: "https://focus.example.net",
};
const signedOut = { ...connected, connected: false, accountEmail: null, displayName: null, mediaReady: false };

beforeEach(() => {
  vi.clearAllMocks();
  status.mockResolvedValue(signedOut);
});

describe("useServerAccount", () => {
  it("loads the account status on mount and falls back to disconnected on failure", async () => {
    status.mockRejectedValueOnce(new Error("no tauri"));
    const { result } = renderHook(() => useServerAccount());
    await waitFor(() => expect(status).toHaveBeenCalled());
    expect(result.current.account.connected).toBe(false);
    expect(result.current.account.available).toBe(false);
  });

  it("reports success and applies the account returned by sign-in", async () => {
    login.mockResolvedValue(connected);
    status.mockResolvedValue(connected);
    const { result } = renderHook(() => useServerAccount());

    let ok = false;
    await act(async () => {
      ok = await result.current.signIn("user@example.com", "StrongPass123");
    });

    expect(ok).toBe(true);
    expect(login).toHaveBeenCalledWith("user@example.com", "StrongPass123");
    expect(result.current.account.connected).toBe(true);
    expect(result.current.error).toBe(false);
    expect(result.current.busy).toBe(false);
  });

  it("reports failure and raises the error flag without changing the account", async () => {
    login.mockRejectedValue(new Error("bad credentials"));
    const { result } = renderHook(() => useServerAccount());

    let ok = true;
    await act(async () => {
      ok = await result.current.signIn("user@example.com", "wrong");
    });

    expect(ok).toBe(false);
    expect(result.current.error).toBe(true);
    expect(result.current.account.connected).toBe(false);
    expect(result.current.busy).toBe(false);
  });

  it("clears the previous error when the next attempt starts", async () => {
    login.mockRejectedValueOnce(new Error("bad credentials")).mockResolvedValueOnce(connected);
    const { result } = renderHook(() => useServerAccount());

    await act(async () => void (await result.current.signIn("user@example.com", "wrong")));
    expect(result.current.error).toBe(true);
    await act(async () => void (await result.current.signIn("user@example.com", "right")));
    expect(result.current.error).toBe(false);
  });

  it("registers with the privacy consent flag", async () => {
    register.mockResolvedValue(connected);
    const { result } = renderHook(() => useServerAccount());

    await act(async () => void (await result.current.register("user@example.com", "StrongPass123", "User", true)));

    expect(register).toHaveBeenCalledWith("user@example.com", "StrongPass123", "User", true);
  });

  it("signs the account out locally after disconnect and delete", async () => {
    status.mockResolvedValue(connected);
    const { result } = renderHook(() => useServerAccount());
    await waitFor(() => expect(result.current.account.connected).toBe(true));

    status.mockResolvedValue(signedOut);
    await act(async () => void (await result.current.disconnect()));
    expect(disconnect).toHaveBeenCalledOnce();
    await waitFor(() => expect(result.current.account.connected).toBe(false));

    status.mockResolvedValue(connected);
    await act(async () => void (await result.current.deleteAccount("StrongPass123")));
    expect(deleteAccount).toHaveBeenCalledWith("StrongPass123");
  });
});

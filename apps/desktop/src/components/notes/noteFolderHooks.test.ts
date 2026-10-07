import { afterEach, describe, expect, it, vi } from "vitest";
import { act, renderHook } from "@testing-library/react";
import type { FormEvent } from "react";
import { useMobileSheetLock } from "./useMobileSheetLock";
import { useNoteFolderCreate } from "./useNoteFolderCreate";

const submitEvent = { preventDefault: vi.fn() } as unknown as FormEvent;

afterEach(() => {
  document.body.style.overflow = "";
});

describe("useNoteFolderCreate", () => {
  it("creates a folder from the trimmed draft and resets the form", () => {
    const onCreate = vi.fn();
    const { result } = renderHook(() => useNoteFolderCreate(onCreate));

    act(() => result.current.toggleCreating());
    act(() => result.current.setDraft("  Work  "));
    act(() => result.current.submit(submitEvent));

    expect(onCreate).toHaveBeenCalledWith("Work");
    expect(result.current.draft).toBe("");
    expect(result.current.creating).toBe(false);
  });

  it("ignores a blank draft and keeps the form open", () => {
    const onCreate = vi.fn();
    const { result } = renderHook(() => useNoteFolderCreate(onCreate));

    act(() => result.current.toggleCreating());
    act(() => result.current.setDraft("   "));
    act(() => result.current.submit(submitEvent));

    expect(onCreate).not.toHaveBeenCalled();
    expect(result.current.creating).toBe(true);
  });

  it("cancels and clears the draft", () => {
    const { result } = renderHook(() => useNoteFolderCreate(vi.fn()));

    act(() => result.current.toggleCreating());
    act(() => result.current.setDraft("Idea"));
    act(() => result.current.cancelCreate());

    expect(result.current.draft).toBe("");
    expect(result.current.creating).toBe(false);
  });
});

describe("useMobileSheetLock", () => {
  it("locks body scroll while active, closes on Escape and restores overflow", () => {
    document.body.style.overflow = "auto";
    const onEscape = vi.fn();
    const { rerender, unmount } = renderHook(({ active }) => useMobileSheetLock(active, onEscape), {
      initialProps: { active: true },
    });

    expect(document.body.style.overflow).toBe("hidden");
    window.dispatchEvent(new KeyboardEvent("keydown", { key: "Escape" }));
    expect(onEscape).toHaveBeenCalledOnce();

    rerender({ active: false });
    expect(document.body.style.overflow).toBe("auto");
    window.dispatchEvent(new KeyboardEvent("keydown", { key: "Escape" }));
    expect(onEscape).toHaveBeenCalledOnce();
    unmount();
  });

  it("does nothing while inactive", () => {
    document.body.style.overflow = "auto";
    renderHook(() => useMobileSheetLock(false, vi.fn()));
    expect(document.body.style.overflow).toBe("auto");
  });
});

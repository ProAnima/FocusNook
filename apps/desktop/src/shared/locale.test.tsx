import { cleanup, render, waitFor } from "@testing-library/react";
import { afterEach, describe, expect, it } from "vitest";
import { LocaleProvider } from "./locale";

describe("LocaleProvider", () => {
  afterEach(() => {
    cleanup();
    window.history.replaceState({}, "", "/");
  });

  it("keeps the document language aligned with the active locale", async () => {
    window.history.replaceState({}, "", "/?localePreview=ja");
    render(<LocaleProvider><span>content</span></LocaleProvider>);

    await waitFor(() => expect(document.documentElement.lang).toBe("ja"));
  });
});

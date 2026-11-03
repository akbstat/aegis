import "@testing-library/jest-dom/vitest";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { cleanup, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { AegisI18nProvider } from "@aegis/ui/i18n";
import { AegisThemeProvider } from "@aegis/ui/theme";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));

import { renderWithFullRouter } from "../../helpers/file-route-utils";
import { mockCommands } from "../../helpers/tauri-mock";
import { TestQueryProvider } from "../../helpers/test-query-provider";

function createMemoryStorage(): Storage {
  const data = new Map<string, string>();
  return {
    get length() {
      return data.size;
    },
    clear() {
      data.clear();
    },
    getItem(key: string) {
      return data.has(key) ? data.get(key)! : null;
    },
    key(index: number) {
      return Array.from(data.keys())[index] ?? null;
    },
    removeItem(key: string) {
      data.delete(key);
    },
    setItem(key: string, value: string) {
      data.set(key, value);
    },
  } as unknown as Storage;
}

beforeEach(() => {
  vi.unstubAllGlobals();
  vi.stubGlobal("localStorage", createMemoryStorage());
});
afterEach(() => cleanup());

function renderRoot() {
  return renderWithFullRouter({
    initialEntries: ["/"],
    wrapper: ({ children }) => (
      <AegisThemeProvider>
        <TestQueryProvider>
          <AegisI18nProvider>{children}</AegisI18nProvider>
        </TestQueryProvider>
      </AegisThemeProvider>
    ),
  });
}

describe("AppLayout — Templates sub-menu entry", () => {
  it("shows Templates under Knowledge Base", async () => {
    mockCommands({ is_logged_in: () => true });
    await renderRoot();
    await userEvent.click(await screen.findByText("Knowledge Base"));
    expect(screen.getByText("Templates")).toBeInTheDocument();
  });

  it("navigates to /templates when the Templates sub-menu is clicked", async () => {
    mockCommands({ is_logged_in: () => true });
    const { router } = await renderRoot();
    await userEvent.click(await screen.findByText("Knowledge Base"));
    await userEvent.click(screen.getByText("Templates"));
    await waitFor(() =>
      expect(router.state.location.pathname).toBe("/templates"),
    );
  });
});

import "@testing-library/jest-dom/vitest";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { cleanup, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { AegisI18nProvider } from "@aegis/ui/i18n";
import { AegisThemeProvider } from "@aegis/ui/theme";

import { renderInRouter } from "../../helpers/file-route-utils";
import { TemplatesPage } from "../../../features/templates";

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

function renderTemplates() {
  return renderInRouter(
    <AegisThemeProvider>
      <AegisI18nProvider>
        <TemplatesPage />
      </AegisI18nProvider>
    </AegisThemeProvider>,
  );
}

describe("TemplatesPage", () => {
  it("renders the heading and the one card", async () => {
    await renderTemplates();
    expect(
      screen.getByRole("heading", { level: 4, name: /templates/i }),
    ).toBeInTheDocument();
    expect(
      screen.getByText("Standard Project Folder Structure"),
    ).toBeInTheDocument();
    expect(
      screen.getByText(
        /Statistical programming standard project folder structure/i,
      ),
    ).toBeInTheDocument();
  });

  it("navigates to /templates/structure when the card is clicked", async () => {
    const { router } = await renderTemplates();
    await userEvent.click(
      screen.getByText("Standard Project Folder Structure"),
    );
    await waitFor(() =>
      expect(router.state.location.pathname).toBe("/templates/structure"),
    );
  });
});

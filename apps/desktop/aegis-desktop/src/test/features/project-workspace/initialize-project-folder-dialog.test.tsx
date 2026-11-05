import "@testing-library/jest-dom/vitest";
import { cleanup, render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import type { ReactNode } from "react";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));

import { invoke } from "@tauri-apps/api/core";

import { AegisThemeProvider } from "@aegis/ui/theme";
import { AegisI18nProvider } from "@aegis/ui/i18n";
import { InitializeProjectFolderDialog } from "../../../features/project-workspace/components/InitializeProjectFolderDialog";

const TEMPLATES = [
  {
    id: 1,
    name: "Standard Project Folder Structure",
    rootpath: "/templates/standard",
    level: "template",
    projectCode: null,
    templateId: null,
  },
  {
    id: 2,
    name: "Minimal",
    rootpath: "/templates/minimal",
    level: "template",
    projectCode: null,
    templateId: null,
  },
];

function wrapperFor(client: QueryClient) {
  return function Wrapper({ children }: { children: ReactNode }) {
    return (
      <AegisThemeProvider>
        <AegisI18nProvider>
          <QueryClientProvider client={client}>{children}</QueryClientProvider>
        </AegisI18nProvider>
      </AegisThemeProvider>
    );
  };
}

function renderDialog(overrides: {
  onClose?: () => void;
  onInitialized?: (structureName: string) => void;
} = {}) {
  const onClose = overrides.onClose ?? vi.fn();
  const onInitialized = overrides.onInitialized ?? vi.fn();
  const client = new QueryClient({
    defaultOptions: {
      queries: { retry: false },
      mutations: { retry: false },
    },
  });
  render(
    <InitializeProjectFolderDialog
      open
      onClose={onClose}
      onInitialized={onInitialized}
    />,
    { wrapper: wrapperFor(client) },
  );
  return { onClose, onInitialized };
}

beforeEach(() => {
  vi.mocked(invoke).mockReset();
  vi.mocked(invoke).mockResolvedValue(TEMPLATES);
});
afterEach(() => {
  cleanup();
  vi.mocked(invoke).mockReset();
});

describe("InitializeProjectFolderDialog", () => {
  it("lists the template structures", async () => {
    renderDialog();
    await waitFor(() =>
      expect(invoke).toHaveBeenCalledWith("list_structures", {
        level: "template",
      }),
    );
    await userEvent.click(screen.getByRole("combobox"));
    expect(
      await screen.findByRole("option", {
        name: "Standard Project Folder Structure",
      }),
    ).toBeInTheDocument();
    expect(screen.getByRole("option", { name: "Minimal" })).toBeInTheDocument();
  });

  it("keeps confirm disabled until a structure is selected", async () => {
    renderDialog();
    const confirm = await screen.findByRole("button", { name: /confirm/i });
    expect(confirm).toBeDisabled();

    await userEvent.click(screen.getByRole("combobox"));
    await userEvent.click(
      await screen.findByRole("option", { name: "Minimal" }),
    );
    expect(screen.getByRole("button", { name: /confirm/i })).toBeEnabled();
  });

  it("reports the selected structure and closes", async () => {
    const { onClose, onInitialized } = renderDialog();
    await userEvent.click(screen.getByRole("combobox"));
    await userEvent.click(
      await screen.findByRole("option", {
        name: "Standard Project Folder Structure",
      }),
    );
    await userEvent.click(screen.getByRole("button", { name: /confirm/i }));

    expect(onInitialized).toHaveBeenCalledWith(
      "Standard Project Folder Structure",
    );
    expect(onClose).toHaveBeenCalled();
  });

  it("cancels without reporting a structure", async () => {
    const { onClose, onInitialized } = renderDialog();
    await userEvent.click(screen.getByRole("button", { name: /cancel/i }));
    expect(onClose).toHaveBeenCalled();
    expect(onInitialized).not.toHaveBeenCalled();
  });

  it("makes no request beyond the structure list", async () => {
    const { onInitialized } = renderDialog();
    await userEvent.click(screen.getByRole("combobox"));
    await userEvent.click(
      await screen.findByRole("option", { name: "Minimal" }),
    );
    await userEvent.click(screen.getByRole("button", { name: /confirm/i }));

    expect(onInitialized).toHaveBeenCalled();
    const commands = vi
      .mocked(invoke)
      .mock.calls.map((call) => call[0])
      .filter((cmd) => cmd !== "list_structures");
    expect(commands).toEqual([]);
  });
});

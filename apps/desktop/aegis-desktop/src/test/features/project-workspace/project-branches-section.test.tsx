import "@testing-library/jest-dom/vitest";
import { cleanup, render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import type { ReactNode } from "react";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));

// The active branch lives in the settings store.
const store = new Map<string, unknown>();
vi.mock("@tauri-apps/plugin-store", () => ({
  load: () =>
    Promise.resolve({
      get: (k: string) => Promise.resolve(store.get(k)),
      set: (k: string, v: unknown) => {
        store.set(k, v);
        return Promise.resolve();
      },
      save: () => Promise.resolve(),
    }),
}));

import { invoke } from "@tauri-apps/api/core";

import { AegisThemeProvider } from "@aegis/ui/theme";
import { AegisI18nProvider } from "@aegis/ui/i18n";
import { ProjectBranchesSection } from "../../../features/project-workspace/components/ProjectBranchesSection";

const BRANCHES = [
  {
    id: 4,
    projectId: 9,
    branch: "2026Q1",
    createdAt: "2026-01-02T03:04:05Z",
    updatedAt: "2026-01-02T03:04:05Z",
  },
  {
    id: 5,
    projectId: 9,
    branch: "2026Q2",
    createdAt: "2026-01-02T03:04:05Z",
    updatedAt: "2026-01-02T03:04:05Z",
  },
];

const TEMPLATES = [
  {
    id: 1,
    name: "Standard Project Folder Structure",
    rootpath: "/templates/standard",
    level: "template",
    projectCode: null,
    templateId: null,
  },
];

/**
 * Answer each Tauri command by name. A handler that throws becomes a
 * rejected promise, the way a failing command surfaces to the caller.
 */
function mockInvokeWith(
  branches: unknown[],
  handlers: Record<string, () => unknown> = {},
) {
  vi.mocked(invoke).mockImplementation((cmd: string) => {
    if (cmd === "list_project_branches") return Promise.resolve(branches);
    if (cmd === "list_structures") return Promise.resolve(TEMPLATES);
    const handler = handlers[cmd];
    if (!handler) {
      return Promise.reject(new Error(`unexpected tauri command: ${cmd}`));
    }
    try {
      return Promise.resolve(handler());
    } catch (e) {
      return Promise.reject(e);
    }
  });
}

function renderSection(
  isLeader = true,
  branches: unknown[] = BRANCHES,
  handlers: Record<string, () => unknown> = {},
) {
  mockInvokeWith(branches, handlers);
  const client = new QueryClient({
    defaultOptions: {
      queries: { retry: false },
      mutations: { retry: false },
    },
  });
  const Wrapper = ({ children }: { children: ReactNode }) => (
    <AegisThemeProvider>
      <AegisI18nProvider>
        <QueryClientProvider client={client}>{children}</QueryClientProvider>
      </AegisI18nProvider>
    </AegisThemeProvider>
  );
  render(<ProjectBranchesSection projectCode="pr1" isLeader={isLeader} />, {
    wrapper: Wrapper,
  });
}

function invokedCommands() {
  return vi.mocked(invoke).mock.calls.map((call) => call[0]);
}

beforeEach(() => {
  vi.mocked(invoke).mockReset();
  store.clear();
});
afterEach(() => {
  cleanup();
  vi.mocked(invoke).mockReset();
});

describe("ProjectBranchesSection", () => {
  it("lists the project's branches", async () => {
    renderSection();
    expect(await screen.findByText("2026Q1")).toBeInTheDocument();
    expect(screen.getByText("2026Q2")).toBeInTheDocument();
  });

  it("checks the switch only for the active branch", async () => {
    store.set("activeBranch.pr1", 4);
    renderSection();
    await waitFor(() =>
      expect(screen.getByRole("switch", { name: /2026Q1/ })).toBeChecked(),
    );
    expect(screen.getByRole("switch", { name: /2026Q2/ })).not.toBeChecked();
  });

  it("makes a branch active without calling the server", async () => {
    renderSection();
    await waitFor(() =>
      expect(screen.getByRole("switch", { name: /2026Q2/ })).toBeInTheDocument(),
    );
    await userEvent.click(screen.getByRole("switch", { name: /2026Q2/ }));

    await waitFor(() => expect(store.get("activeBranch.pr1")).toBe(5));
    expect(
      invokedCommands().filter((cmd) => cmd !== "list_project_branches"),
    ).toEqual([]);
  });

  it("deactivates the active branch", async () => {
    store.set("activeBranch.pr1", 4);
    renderSection();
    await waitFor(() =>
      expect(screen.getByRole("switch", { name: /2026Q1/ })).toBeChecked(),
    );
    await userEvent.click(screen.getByRole("switch", { name: /2026Q1/ }));

    await waitFor(() => expect(store.get("activeBranch.pr1")).toBeNull());
  });

  it("hides the action column from a non-leader", async () => {
    renderSection(false);
    await screen.findByText("2026Q1");
    expect(screen.queryByTestId("branch-create")).not.toBeInTheDocument();
    expect(screen.queryByTestId("branch-rename-4")).not.toBeInTheDocument();
    expect(
      screen.queryByTestId("branch-initialize-4"),
    ).not.toBeInTheDocument();
    // The switch stays available to everyone.
    expect(screen.getByRole("switch", { name: /2026Q1/ })).toBeInTheDocument();
  });

  it("shows the action column to a leader", async () => {
    renderSection(true);
    expect(await screen.findByTestId("branch-create")).toBeInTheDocument();
    expect(screen.getByTestId("branch-rename-4")).toBeInTheDocument();
    expect(screen.getByTestId("branch-initialize-4")).toBeInTheDocument();
  });

  it("renders an empty state for a project with no branches", async () => {
    renderSection(true, []);
    expect(await screen.findByText("No branches yet")).toBeInTheDocument();
  });

  it("leaves every switch unchecked for a stale active branch", async () => {
    store.set("activeBranch.pr1", 999);
    renderSection();
    await waitFor(() =>
      expect(
        screen.getByRole("switch", { name: /2026Q1/ }),
      ).not.toBeChecked(),
    );
    expect(screen.getByRole("switch", { name: /2026Q2/ })).not.toBeChecked();
  });

  it("pre-fills the rename dialog with the current name", async () => {
    renderSection();
    await userEvent.click(await screen.findByTestId("branch-rename-4"));
    expect(await screen.findByLabelText("Branch name")).toHaveValue("2026Q1");
  });

  it("disables confirm for a whitespace-only name", async () => {
    renderSection();
    await userEvent.click(await screen.findByTestId("branch-create"));
    const input = await screen.findByLabelText("Branch name");
    await userEvent.type(input, "   ");
    expect(screen.getByRole("button", { name: /confirm/i })).toBeDisabled();

    await userEvent.type(input, "2026Q3");
    expect(screen.getByRole("button", { name: /confirm/i })).toBeEnabled();
  });

  it("shows a duplicate-name error and keeps the dialog open", async () => {
    renderSection(true, BRANCHES, {
      create_project_branch: () => {
        throw {
          kind: "http",
          status: 409,
          code: "duplicate_branch",
          message: "branch already exists: 2026Q1",
        };
      },
    });
    await userEvent.click(await screen.findByTestId("branch-create"));
    await userEvent.type(await screen.findByLabelText("Branch name"), "2026Q1");
    await userEvent.click(screen.getByRole("button", { name: /confirm/i }));

    expect(await screen.findByText(/duplicate_branch/)).toBeInTheDocument();
    expect(screen.getByLabelText("Branch name")).toBeInTheDocument();
  });

  it("shows a forbidden error and keeps the dialog open", async () => {
    // The server gates branch writes behind admin/root, so a project
    // leader who is not an admin gets `403` on every rename.
    renderSection(true, BRANCHES, {
      update_project_branch: () => {
        throw {
          kind: "http",
          status: 403,
          code: "forbidden",
          message: "admin or root required",
        };
      },
    });
    await userEvent.click(await screen.findByTestId("branch-rename-4"));
    await userEvent.clear(await screen.findByLabelText("Branch name"));
    await userEvent.type(screen.getByLabelText("Branch name"), "2026Q9");
    await userEvent.click(screen.getByRole("button", { name: /confirm/i }));

    expect(await screen.findByText(/forbidden/)).toBeInTheDocument();
    expect(screen.getByLabelText("Branch name")).toBeInTheDocument();
  });

  it("reports the initialization with a success message", async () => {
    renderSection();
    await userEvent.click(await screen.findByTestId("branch-initialize-4"));
    await userEvent.click(await screen.findByRole("combobox"));
    await userEvent.click(
      await screen.findByRole("option", {
        name: "Standard Project Folder Structure",
      }),
    );
    await userEvent.click(screen.getByRole("button", { name: /confirm/i }));

    expect(await screen.findByText("Initialize successfully")).toBeInTheDocument();
  });
});

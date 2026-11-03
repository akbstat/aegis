import "@testing-library/jest-dom/vitest";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { cleanup, screen, waitFor, within } from "@testing-library/react";
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

const STRUCTURE = {
  id: 3,
  name: "Standard",
  level: "template",
  projectCode: null,
  templateId: null,
  nodes: [
    {
      nodeId: "d1",
      description: "raw data",
      path: "raw",
      directory: true,
      parent: null,
      reference: null,
      migration: null,
      children: [
        {
          nodeId: "d2",
          description: "",
          path: "raw/data",
          directory: true,
          parent: "d1",
          reference: null,
          migration: null,
          children: [
            {
              nodeId: "f1",
              description: "",
              path: "raw/data/a.sas7bdat",
              directory: false,
              parent: "d2",
              reference: "/abs/a.sas7bdat",
              migration: null,
              children: [],
            },
          ],
        },
      ],
    },
  ],
};

const TEMPLATES = [
  {
    id: 3,
    name: "Standard",
    level: "template",
    projectCode: null,
    templateId: null,
  },
];

const GENERAL_USER = {
  id: 1,
  code: "alice",
  name: "Alice",
  role: "general",
  active: true,
  createdAt: "2026-01-01T00:00:00Z",
  updatedAt: "2026-01-01T00:00:00Z",
};

const ADMIN_USER = { ...GENERAL_USER, role: "admin" };

function baseCommands(extra: Record<string, unknown> = {}) {
  return {
    is_logged_in: () => true,
    current_user: () => GENERAL_USER,
    list_structures: () => TEMPLATES,
    get_structure: () => STRUCTURE,
    ...extra,
  };
}

beforeEach(() => {
  vi.unstubAllGlobals();
  vi.stubGlobal("localStorage", createMemoryStorage());
});
afterEach(() => cleanup());

function renderPage(path = "/templates/structure/3") {
  return renderWithFullRouter({
    initialEntries: [path],
    wrapper: ({ children }) => (
      <AegisThemeProvider>
        <TestQueryProvider>
          <AegisI18nProvider>{children}</AegisI18nProvider>
        </TestQueryProvider>
      </AegisThemeProvider>
    ),
  });
}

describe("StructureTemplatePage", () => {
  it("lists the template structures in the sidebar", async () => {
    mockCommands(baseCommands());
    await renderPage();
    const sidebar = await screen.findByTestId("structure-sidebar");
    expect(within(sidebar).getByText("Standard")).toBeInTheDocument();
  });

  it("renders the tree of the selected structure", async () => {
    mockCommands(baseCommands());
    await renderPage();
    expect(await screen.findByText("raw")).toBeInTheDocument();
    expect(await screen.findByText("raw/data")).toBeInTheDocument();
  });

  it("switches to the flat view, where a sub-directory is not nested", async () => {
    mockCommands(baseCommands());
    await renderPage();
    await userEvent.click(await screen.findByRole("button", { name: "Flat" }));
    // The flat view shows the directory by its own name, plus its full path.
    await waitFor(() => expect(screen.getByText("data")).toBeInTheDocument());
    expect(screen.getByText("raw/data")).toBeInTheDocument();
  });

  it("shows an empty state when the structure has no nodes", async () => {
    mockCommands(
      baseCommands({
        get_structure: () => ({ ...STRUCTURE, nodes: [] }),
      }),
    );
    await renderPage();
    expect(
      await screen.findByText("No folder structure yet"),
    ).toBeInTheDocument();
  });

  it("shows an empty state and does not select anything when there are no templates", async () => {
    mockCommands(
      baseCommands({
        list_structures: () => [],
      }),
    );
    await renderPage("/templates/structure");
    expect(await screen.findByText("No templates yet")).toBeInTheDocument();
  });

  it("hides the node actions from a general user", async () => {
    mockCommands(baseCommands());
    await renderPage();
    expect(await screen.findByText("raw")).toBeInTheDocument();
    expect(
      screen.queryAllByRole("button", { name: /node actions/i }),
    ).toHaveLength(0);
  });

  it("shows the node actions to an admin", async () => {
    mockCommands(
      baseCommands({
        current_user: () => ADMIN_USER,
      }),
    );
    await renderPage();
    expect(
      (await screen.findAllByRole("button", { name: /node actions/i })).length,
    ).toBeGreaterThan(0);
  });
});

import "@testing-library/jest-dom/vitest";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import {
  cleanup,
  fireEvent,
  screen,
  waitFor,
  within,
} from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { AegisI18nProvider } from "@aegis/ui/i18n";
import { AegisThemeProvider } from "@aegis/ui/theme";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));

import { renderWithFullRouter } from "../../helpers/file-route-utils";
import { httpError, mockCommands } from "../../helpers/tauri-mock";
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
      entryName: "raw",
      directory: true,
      parent: null,
      reference: null,
      migration: null,
      children: [
        {
          nodeId: "d2",
          description: "",
          entryName: "data",
          directory: true,
          parent: "d1",
          reference: null,
          migration: null,
          children: [
            {
              nodeId: "f1",
              description: "",
              // The name differs from the reference's file name on
              // purpose, so the two are not mistaken for each other.
              entryName: "b.sas7bdat",
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
    expect(await screen.findByText("data")).toBeInTheDocument();
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

  it("shows the files under each directory in the flat view", async () => {
    mockCommands(baseCommands());
    await renderPage();
    await userEvent.click(await screen.findByRole("button", { name: "Flat" }));
    await waitFor(() => expect(screen.getByText("data")).toBeInTheDocument());
    // The flat view exists to show a directory's files as its leaves, so
    // they must be visible without expanding the directory first.
    expect(screen.getByText("a.sas7bdat")).toBeInTheDocument();
  });

  it("does not show the empty state while the structure is loading", async () => {
    let resolveStructure: (value: unknown) => void = () => {};
    mockCommands(
      baseCommands({
        get_structure: () =>
          new Promise((resolve) => {
            resolveStructure = resolve;
          }),
      }),
    );
    await renderPage();
    expect(screen.queryByText("No folder structure yet")).not.toBeInTheDocument();
    resolveStructure(STRUCTURE);
    await waitFor(() => expect(screen.getByText("raw")).toBeInTheDocument());
  });

  it("does not offer to add a root node when no structure is selected", async () => {
    mockCommands(
      baseCommands({
        list_structures: () => [],
        current_user: () => ADMIN_USER,
      }),
    );
    await renderPage("/templates/structure");
    // Wait for the admin role to land — the app shell shows "Management"
    // for an admin — so the assertion is not made before the user query
    // has resolved.
    await screen.findByText("Management");
    expect(
      screen.queryByRole("button", { name: /add root node/i }),
    ).not.toBeInTheDocument();
  });

  it("does not show a stale save error in a freshly opened drawer", async () => {
    mockCommands(
      baseCommands({
        add_structure_node: () => {
          throw httpError(400, "validation_failed", "bad path");
        },
        current_user: () => ADMIN_USER,
      }),
    );
    await renderPage();
    await userEvent.click(
      await screen.findByRole("button", { name: /add root node/i }),
    );
    await userEvent.type(screen.getByLabelText(/^Entry name/), "raw");
    await userEvent.click(screen.getByRole("button", { name: "Create" }));
    expect(await screen.findByText(/Save failed/)).toBeInTheDocument();

    await userEvent.click(screen.getByRole("button", { name: "Cancel" }));
    await userEvent.click(
      await screen.findByRole("button", { name: /add root node/i }),
    );
    expect(screen.queryByText(/Save failed/)).not.toBeInTheDocument();
  });

  it("creates a template structure from the sidebar", async () => {
    const createStructure = vi.fn(() => ({
      id: 9,
      name: "Fresh",
      level: "template",
      projectCode: null,
      templateId: null,
      nodes: [],
    }));
    mockCommands(
      baseCommands({
        current_user: () => ADMIN_USER,
        create_structure: createStructure,
      }),
    );
    await renderPage();
    await userEvent.click(
      await screen.findByRole("button", { name: /add template/i }),
    );
    await userEvent.type(screen.getByLabelText(/^Name/), "Fresh");
    await userEvent.click(screen.getByRole("button", { name: "Create" }));
    await waitFor(() => expect(createStructure).toHaveBeenCalled());
    expect(createStructure).toHaveBeenCalledWith({
      body: {
        name: "Fresh",
        level: "template",
        projectCode: null,
        templateId: null,
      },
    });
  });

  it("renames a structure from the sidebar", async () => {
    const renameStructure = vi.fn(() => ({
      id: 3,
      name: "Renamed",
      level: "template",
      projectCode: null,
      templateId: null,
      nodes: [],
    }));
    mockCommands(
      baseCommands({
        current_user: () => ADMIN_USER,
        rename_structure: renameStructure,
      }),
    );
    await renderPage();
    await userEvent.click(
      await screen.findByRole("button", { name: /rename template/i }),
    );
    await userEvent.clear(screen.getByLabelText(/^Name/));
    await userEvent.type(screen.getByLabelText(/^Name/), "Renamed");
    await userEvent.click(screen.getByRole("button", { name: "Save" }));
    await waitFor(() => expect(renameStructure).toHaveBeenCalled());
    expect(renameStructure).toHaveBeenCalledWith({
      id: 3,
      body: { name: "Renamed" },
    });
  });

  it("deletes a structure after the confirmation", async () => {
    const deleteStructure = vi.fn();
    mockCommands(
      baseCommands({
        current_user: () => ADMIN_USER,
        delete_structure: deleteStructure,
      }),
    );
    await renderPage();
    await userEvent.click(
      await screen.findByRole("button", { name: /delete template/i }),
    );
    expect(
      await screen.findByText(/Delete "Standard" and all of its folders?/),
    ).toBeInTheDocument();
    await userEvent.click(screen.getByRole("button", { name: "Confirm" }));
    await waitFor(() => expect(deleteStructure).toHaveBeenCalled());
    expect(deleteStructure).toHaveBeenCalledWith({ id: 3 });
  });

  it("moves off a structure that was just deleted", async () => {
    // The server drops the deleted structure from the list, so the
    // mock does too — otherwise the refetch would put it straight back.
    let deleted = false;
    mockCommands(
      baseCommands({
        current_user: () => ADMIN_USER,
        list_structures: () => (deleted ? [] : TEMPLATES),
        delete_structure: () => {
          deleted = true;
        },
      }),
    );
    const { router } = await renderPage();
    await userEvent.click(
      await screen.findByRole("button", { name: /delete template/i }),
    );
    await userEvent.click(screen.getByRole("button", { name: "Confirm" }));
    // The deleted structure was the one on screen — the page must not
    // stay pointed at a structure that no longer exists.
    await waitFor(() =>
      expect(router.state.location.pathname).toBe("/templates/structure"),
    );
  });

  it("keeps the view toggle and the add button out of the scrolling tree", async () => {
    mockCommands(baseCommands({ current_user: () => ADMIN_USER }));
    await renderPage();

    const scroller = await screen.findByTestId("structure-tree-scroll");
    // The tree scrolls on its own; the toggle and the add button sit
    // above it and must not scroll away with it.
    expect(scroller.contains(screen.getByRole("button", { name: "Complete" }))).toBe(false);
    expect(
      scroller.contains(screen.getByRole("button", { name: /add root node/i })),
    ).toBe(false);
    // ...while the tree itself is inside it.
    expect(scroller.contains(screen.getByText("raw"))).toBe(true);
  });

  it("does not collapse the node when a menu item is clicked", async () => {
    mockCommands(baseCommands({ current_user: () => ADMIN_USER }));
    await renderPage();
    const item = (await screen.findByText("raw")).closest("li");

    await userEvent.click(
      (await screen.findAllByRole("button", { name: /node actions/i }))[0],
    );
    await userEvent.click(screen.getByText("Add node"));

    // The menu renders in a portal, but React portals bubble events
    // through the React tree — so the click must not reach the tree
    // item, which would collapse it.
    expect(item).toHaveAttribute("aria-expanded", "true");
  });

  it("does not collapse the node when the menu is dismissed", async () => {
    mockCommands(baseCommands({ current_user: () => ADMIN_USER }));
    await renderPage();
    const item = (await screen.findByText("raw")).closest("li");

    await userEvent.click(
      (await screen.findAllByRole("button", { name: /node actions/i }))[0],
    );
    const backdrop = document.querySelector(".MuiBackdrop-root");
    expect(backdrop).not.toBeNull();
    fireEvent.click(backdrop!);

    expect(item).toHaveAttribute("aria-expanded", "true");
  });

  it("does not collapse the node when the action menu is opened", async () => {
    mockCommands(baseCommands({ current_user: () => ADMIN_USER }));
    await renderPage();
    // The tree renders fully expanded, so the child is on screen.
    expect(await screen.findByText("data")).toBeInTheDocument();
    const item = (await screen.findByText("raw")).closest("li");
    expect(item).toHaveAttribute("aria-expanded", "true");

    await userEvent.click(
      (await screen.findAllByRole("button", { name: /node actions/i }))[0],
    );

    // The click must not reach the tree item, which would collapse it.
    expect(item).toHaveAttribute("aria-expanded", "true");
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

import "@testing-library/jest-dom/vitest";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { cleanup, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { AegisI18nProvider } from "@aegis/ui/i18n";
import { AegisThemeProvider } from "@aegis/ui/theme";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));

// The branches section reads the active branch from the settings store.
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
import { TestQueryProvider } from "../../../test/helpers/test-query-provider";
import { renderWithFullRouter } from "../../../test/helpers/file-route-utils";
import { mockCommands } from "../../../test/helpers/tauri-mock";
import type { ProjectView, UserView } from "../../../shared/api";
import { makeProject } from "../../helpers/project-fixture";

const leader: UserView = {
  id: 1,
  code: "alice",
  name: "Alice",
  role: "admin",
  active: true,
  createdAt: "2026-01-01T00:00:00Z",
  updatedAt: "2026-01-01T00:00:00Z",
};

const other: UserView = {
  id: 2,
  code: "carol",
  name: "Carol",
  role: "general",
  active: true,
  createdAt: "2026-01-01T00:00:00Z",
  updatedAt: "2026-01-01T00:00:00Z",
};

const projectFixture: ProjectView = makeProject({
  code: "alpha",
  members: {
    leaders: [{ code: "alice", name: "Alice" }],
    workers: [{ code: "bob", name: "Bob" }],
  },
  unblindMembers: { leaders: [], workers: [] },
  configurations: {
    language: "en",
    tags: [{ key: "Product", value: "DEMO-001" }],
  },
});

const sdtmVersionsResponse = {
  versions: [
    {
      id: 1,
      name: "SDTMIG v3.1.2",
      createdAt: "2026-01-01T00:00:00Z",
      updatedAt: "2026-01-01T00:00:00Z",
    },
    {
      id: 2,
      name: "SDTMIG v3.2",
      createdAt: "2026-01-01T00:00:00Z",
      updatedAt: "2026-01-01T00:00:00Z",
    },
    {
      id: 3,
      name: "SDTMIG v3.3",
      createdAt: "2026-01-01T00:00:00Z",
      updatedAt: "2026-01-01T00:00:00Z",
    },
  ],
};

const terminologyVersionsResponse = [
  {
    id: 10,
    kind: "sdtm" as const,
    name: "2024-03-29",
    createdAt: "2026-01-01T00:00:00Z",
    updatedAt: "2026-01-01T00:00:00Z",
  },
  {
    id: 11,
    kind: "sdtm" as const,
    name: "2023-09-29",
    createdAt: "2026-01-01T00:00:00Z",
    updatedAt: "2026-01-01T00:00:00Z",
  },
  // Adam release — must NOT appear as a MenuItem in the picker.
  {
    id: 12,
    kind: "adam" as const,
    name: "ADAM-2024",
    createdAt: "2026-01-01T00:00:00Z",
    updatedAt: "2026-01-01T00:00:00Z",
  },
];

const projectWithSdtmig: ProjectView = makeProject({
  code: "alpha",
  members: {
    leaders: [{ code: "alice", name: "Alice" }],
    workers: [{ code: "bob", name: "Bob" }],
  },
  unblindMembers: { leaders: [], workers: [] },
  configurations: {
    language: "en",
    tags: [{ key: "Product", value: "DEMO-001" }],
    sdtmig: { versionId: 2, versionName: "SDTMIG v3.2" },
  },
});

beforeEach(() => {
  (invoke as unknown as ReturnType<typeof vi.fn>).mockReset();
  store.clear();
  vi.stubGlobal("localStorage", {
    getItem: () => null,
    setItem: () => {},
    removeItem: () => {},
    clear: () => {},
    key: () => null,
    get length() {
      return 0;
    },
  });
});
afterEach(() => cleanup());

function renderPage(initialEntry = "/project/alpha/configuration") {
  return renderWithFullRouter({
    initialEntries: [initialEntry],
    wrapper: ({ children }) => (
      <AegisThemeProvider>
        <TestQueryProvider>
          <AegisI18nProvider>{children}</AegisI18nProvider>
        </TestQueryProvider>
      </AegisThemeProvider>
    ),
  });
}

describe("ProjectConfigurationPage — leader view", () => {
  beforeEach(() => {
    mockCommands({
      is_logged_in: () => true,
      current_user: () => leader,
      get_project_by_code: () => projectFixture,
      list_sdtm_versions: () => sdtmVersionsResponse,
      list_terminology_versions: () => terminologyVersionsResponse,
    });
  });

  it("renders the heading", async () => {
    await renderPage();
    expect(
      await screen.findByRole("heading", { name: /configuration/i }),
    ).toBeInTheDocument();
  });

  it("renders the section sidebar with General selected", async () => {
    await renderPage();
    const general = await screen.findByTestId("config-section-general");
    const members = await screen.findByTestId("config-section-members");
    const filepath = await screen.findByTestId("config-section-filepath");
    // MUI ListItemButton uses the `Mui-selected` class for the active
    // item rather than `aria-selected`.
    expect(general.className).toMatch(/Mui-selected/);
    expect(members.className).not.toMatch(/Mui-selected/);
    expect(filepath.className).not.toMatch(/Mui-selected/);
  });

  it("orders the sections General, Members, Branches, Filepath", async () => {
    await renderPage();
    await screen.findByTestId("config-section-general");
    const tabs = screen.getAllByRole("tab");
    expect(tabs.map((tab) => tab.textContent)).toEqual([
      "General",
      "Members",
      "Branches",
      "Filepath",
    ]);
  });

  it("does NOT render the read-only banner for a project leader", async () => {
    await renderPage();
    await screen.findByTestId("config-section-general");
    expect(screen.queryByTestId("config-readonly")).not.toBeInTheDocument();
  });

  it("Save button starts disabled and enables after a tag change", async () => {
    await renderPage();
    const save = await screen.findByTestId("config-general-save");
    expect(save).toBeDisabled();

    // Type into the existing tag's value field to flip `tagsTouched`.
    // TagEditor uses TextField, which userEvent.click+type handles cleanly.
    const valueInput = await screen.findByLabelText(/tag value/i);
    await userEvent.clear(valueInput);
    await userEvent.type(valueInput, "DEMO-002");

    await waitFor(() => expect(save).not.toBeDisabled());
  });

  it("clicking Save fires update_project with configurations { language, tags, sdtmig }", async () => {
    mockCommands({
      is_logged_in: () => true,
      current_user: () => leader,
      get_project_by_code: () => projectFixture,
      list_sdtm_versions: () => sdtmVersionsResponse,
      update_project: () => projectFixture,
    });
    await renderPage();

    // Edit the existing tag's value to flip `tagsTouched`. The whole
    // configurations object is then sent on save.
    const valueInput = await screen.findByLabelText(/tag value/i);
    await userEvent.clear(valueInput);
    await userEvent.type(valueInput, "DEMO-002");

    const save = await screen.findByTestId("config-general-save");
    await waitFor(() => expect(save).not.toBeDisabled());
    await userEvent.click(save);

    await waitFor(() =>
      expect(invoke).toHaveBeenCalledWith(
        "update_project",
        expect.objectContaining({
          code: "alpha",
          body: expect.objectContaining({
            configurations: expect.objectContaining({
              language: "en",
              tags: expect.arrayContaining([
                { key: "Product", value: "DEMO-002" },
              ]),
              sdtmig: null,
              sdtmTerminology: null,
            }),
          }),
        }),
      ),
    );
  });

  it("renders the SDTMIG Select with the seeded version preselected", async () => {
    mockCommands({
      is_logged_in: () => true,
      current_user: () => leader,
      get_project_by_code: () => projectWithSdtmig,
      list_sdtm_versions: () => sdtmVersionsResponse,
    });
    await renderPage();
    // The Select's rendered value reflects the saved version's name.
    // MUI Select shows the selected text inside the trigger div;
    // findByText works because the MenuItem children are not in the
    // DOM until the dropdown is opened.
    expect(await screen.findByText("SDTMIG v3.2")).toBeInTheDocument();
  });

  it("renders an '(unspecified)' item as the first option", async () => {
    mockCommands({
      is_logged_in: () => true,
      current_user: () => leader,
      get_project_by_code: () => projectFixture,
      list_sdtm_versions: () => sdtmVersionsResponse,
    });
    await renderPage();
    // Open the Select to render the MenuItem children into the DOM.
    await userEvent.click(await screen.findByLabelText(/sdtmig version/i));
    expect(
      await screen.findByRole("option", { name: /\(unspecified\)/i }),
    ).toBeInTheDocument();
  });

  it("picking a SDTMIG version enables Save", async () => {
    mockCommands({
      is_logged_in: () => true,
      current_user: () => leader,
      get_project_by_code: () => projectFixture,
      list_sdtm_versions: () => sdtmVersionsResponse,
    });
    await renderPage();
    const save = await screen.findByTestId("config-general-save");
    expect(save).toBeDisabled();

    await userEvent.click(await screen.findByLabelText(/sdtmig version/i));
    await userEvent.click(
      await screen.findByRole("option", { name: /sdtmig v3\.2/i }),
    );

    await waitFor(() => expect(save).not.toBeDisabled());
  });

  it("picking '(unspecified)' → Save → body.configurations.sdtmig === null", async () => {
    mockCommands({
      is_logged_in: () => true,
      current_user: () => leader,
      get_project_by_code: () => projectWithSdtmig,
      list_sdtm_versions: () => sdtmVersionsResponse,
      update_project: () => projectFixture,
    });
    await renderPage();

    await userEvent.click(await screen.findByLabelText(/sdtmig version/i));
    await userEvent.click(
      await screen.findByRole("option", { name: /\(unspecified\)/i }),
    );

    const save = await screen.findByTestId("config-general-save");
    await waitFor(() => expect(save).not.toBeDisabled());
    await userEvent.click(save);

    await waitFor(() =>
      expect(invoke).toHaveBeenCalledWith(
        "update_project",
        expect.objectContaining({
          code: "alpha",
          body: expect.objectContaining({
            configurations: expect.objectContaining({
              sdtmig: null,
            }),
          }),
        }),
      ),
    );
  });

  it("clicking the Members sidebar item shows the Members section", async () => {
    await renderPage();
    await userEvent.click(await screen.findByTestId("config-section-members"));
    expect(await screen.findByTestId("config-leaders")).toBeInTheDocument();
    expect(await screen.findByTestId("config-workers")).toBeInTheDocument();
  });

  it("renders the SDTM Terminology Select with the seeded selection when initial.sdtmTerminology is set", async () => {
    mockCommands({
      is_logged_in: () => true,
      current_user: () => leader,
      get_project_by_code: () =>
        makeProject({
          code: "alpha",
          members: {
            leaders: [{ code: "alice", name: "Alice" }],
            workers: [{ code: "bob", name: "Bob" }],
          },
          unblindMembers: { leaders: [], workers: [] },
          configurations: {
            language: "en",
            tags: [{ key: "Product", value: "DEMO-001" }],
            sdtmig: { versionId: 2, versionName: "SDTMIG v3.2" },
            sdtmTerminology: {
              versionId: 10,
              versionName: "2024-03-29",
            },
          },
        }),
      list_sdtm_versions: () => sdtmVersionsResponse,
      list_terminology_versions: () => terminologyVersionsResponse,
    });
    await renderPage();
    expect(await screen.findByText("2024-03-29")).toBeInTheDocument();
  });

  it("SDTM Terminology picker filters out Adam releases", async () => {
    await renderPage();
    await userEvent.click(await screen.findByLabelText(/sdtm terminology/i));
    expect(
      await screen.findByRole("option", { name: "2024-03-29" }),
    ).toBeInTheDocument();
    expect(
      screen.queryByRole("option", { name: "ADAM-2024" }),
    ).not.toBeInTheDocument();
  });

  it("picking a SDTM Terminology release enables Save", async () => {
    await renderPage();
    const save = await screen.findByTestId("config-general-save");
    expect(save).toBeDisabled();

    await userEvent.click(await screen.findByLabelText(/sdtm terminology/i));
    await userEvent.click(
      await screen.findByRole("option", { name: "2024-03-29" }),
    );

    await waitFor(() => expect(save).not.toBeDisabled());
  });

  it("picking '(unspecified)' on SDTM Terminology → Save → body.configurations.sdtmTerminology === null", async () => {
    mockCommands({
      is_logged_in: () => true,
      current_user: () => leader,
      get_project_by_code: () =>
        makeProject({
          code: "alpha",
          members: {
            leaders: [{ code: "alice", name: "Alice" }],
            workers: [{ code: "bob", name: "Bob" }],
          },
          unblindMembers: { leaders: [], workers: [] },
          configurations: {
            language: "en",
            tags: [{ key: "Product", value: "DEMO-001" }],
            sdtmig: { versionId: 2, versionName: "SDTMIG v3.2" },
            sdtmTerminology: {
              versionId: 10,
              versionName: "2024-03-29",
            },
          },
        }),
      list_sdtm_versions: () => sdtmVersionsResponse,
      list_terminology_versions: () => terminologyVersionsResponse,
      update_project: () => projectFixture,
    });
    await renderPage();

    await userEvent.click(await screen.findByLabelText(/sdtm terminology/i));
    await userEvent.click(
      await screen.findByRole("option", { name: /\(unspecified\)/i }),
    );

    const save = await screen.findByTestId("config-general-save");
    await waitFor(() => expect(save).not.toBeDisabled());
    await userEvent.click(save);

    await waitFor(() =>
      expect(invoke).toHaveBeenCalledWith(
        "update_project",
        expect.objectContaining({
          code: "alpha",
          body: expect.objectContaining({
            configurations: expect.objectContaining({
              sdtmTerminology: null,
            }),
          }),
        }),
      ),
    );
  });
});

describe("ProjectConfigurationPage — non-leader view", () => {
  beforeEach(() => {
    mockCommands({
      is_logged_in: () => true,
      current_user: () => other,
      get_project_by_code: () => projectFixture,
      list_sdtm_versions: () => sdtmVersionsResponse,
      list_terminology_versions: () => terminologyVersionsResponse,
    });
  });

  it("renders the read-only banner and hides Save buttons", async () => {
    await renderPage();
    expect(await screen.findByTestId("config-readonly")).toBeInTheDocument();
    expect(
      screen.queryByTestId("config-general-save"),
    ).not.toBeInTheDocument();
    expect(
      screen.queryByTestId("config-members-save"),
    ).not.toBeInTheDocument();
  });

  it("language Select is disabled for non-leaders", async () => {
    await renderPage();
    // The Select wraps in a FormControl; when disabled, the form
    // control root has the `Mui-disabled` class.
    const select = await screen.findByLabelText(/preferred language/i);
    await waitFor(() => {
      const fc = select.closest(".MuiFormControl-root");
      expect(fc).toBeTruthy();
      // The disabled class lands on the inner Select wrapper rather
      // than the form control root; check both possibilities.
      const root = select.closest(".MuiInputBase-root");
      expect(
        fc!.classList.contains("Mui-disabled") ||
          root!.classList.contains("Mui-disabled"),
      ).toBe(true);
    });
  });

  it("SDTMIG Select is disabled for non-leaders", async () => {
    await renderPage();
    const select = await screen.findByLabelText(/sdtmig version/i);
    await waitFor(() => {
      const fc = select.closest(".MuiFormControl-root");
      const root = select.closest(".MuiInputBase-root");
      expect(
        fc!.classList.contains("Mui-disabled") ||
          root!.classList.contains("Mui-disabled"),
      ).toBe(true);
    });
  });

  it("SDTM Terminology Select is disabled for non-leaders", async () => {
    await renderPage();
    const select = await screen.findByLabelText(/sdtm terminology/i);
    await waitFor(() => {
      const fc = select.closest(".MuiFormControl-root");
      const root = select.closest(".MuiInputBase-root");
      expect(
        fc!.classList.contains("Mui-disabled") ||
          root!.classList.contains("Mui-disabled"),
      ).toBe(true);
    });
  });

  it("renders the branches section without the leader-only actions", async () => {
    mockCommands({
      is_logged_in: () => true,
      current_user: () => other,
      get_project_by_code: () => projectFixture,
      list_project_branches: () => [
        {
          id: 4,
          projectId: 9,
          branch: "2026Q1",
          createdAt: "2026-01-02T03:04:05Z",
          updatedAt: "2026-01-02T03:04:05Z",
        },
      ],
    });
    await renderPage();
    await userEvent.click(await screen.findByTestId("config-section-branches"));
    expect(await screen.findByTestId("project-branches-table")).toBeInTheDocument();
    // The switch is available to everyone; creating is not.
    expect(screen.getByRole("switch", { name: /2026Q1/ })).toBeInTheDocument();
    expect(screen.queryByTestId("branch-create")).not.toBeInTheDocument();
  });
});

describe("ProjectConfigurationPage — filepath placeholder", () => {
  beforeEach(() => {
    mockCommands({
      is_logged_in: () => true,
      current_user: () => leader,
      get_project_by_code: () => projectFixture,
      list_sdtm_versions: () => sdtmVersionsResponse,
    });
  });

  it("clicking Filepath shows the placeholder", async () => {
    await renderPage();
    await userEvent.click(await screen.findByTestId("config-section-filepath"));
    expect(
      await screen.findByTestId("config-filepath-placeholder"),
    ).toBeInTheDocument();
  });
});

describe("ProjectConfigurationPage — branches section", () => {
  const branches = [
    {
      id: 4,
      projectId: 9,
      branch: "2026Q1",
      createdAt: "2026-01-02T03:04:05Z",
      updatedAt: "2026-01-02T03:04:05Z",
    },
  ];

  beforeEach(() => {
    mockCommands({
      is_logged_in: () => true,
      current_user: () => leader,
      get_project_by_code: () => projectFixture,
      list_sdtm_versions: () => sdtmVersionsResponse,
      list_terminology_versions: () => terminologyVersionsResponse,
      list_project_branches: () => branches,
    });
  });

  it("clicking Branches shows the branches table", async () => {
    await renderPage();
    await userEvent.click(await screen.findByTestId("config-section-branches"));
    expect(await screen.findByTestId("project-branches-table")).toBeInTheDocument();
    expect(await screen.findByText("2026Q1")).toBeInTheDocument();
  });
});

describe("ProjectConfigurationPage — error path", () => {
  it("renders an Alert when get_project_by_code fails", async () => {
    mockCommands({
      is_logged_in: () => true,
      current_user: () => leader,
      get_project_by_code: () => {
        throw {
          kind: "http",
          status: 500,
          code: "server",
          message: "boom",
        };
      },
    });
    await renderPage();
    expect(await screen.findByRole("alert")).toHaveTextContent(/server/i);
  });
});
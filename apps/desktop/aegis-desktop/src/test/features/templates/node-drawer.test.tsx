import "@testing-library/jest-dom/vitest";
import { afterEach, describe, expect, it, vi } from "vitest";
import { cleanup, render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import type { ComponentProps } from "react";
import { AegisI18nProvider } from "@aegis/ui/i18n";
import { AegisThemeProvider } from "@aegis/ui/theme";

vi.mock("@tauri-apps/plugin-dialog", () => ({ open: vi.fn() }));

import { open } from "@tauri-apps/plugin-dialog";

import type { StructureNodeView } from "../../../shared/api";
import {
  NodeDrawer,
  isValidEntryName,
} from "../../../features/templates/components/NodeDrawer";

afterEach(() => {
  cleanup();
  vi.mocked(open).mockReset();
});

const PARENT: StructureNodeView = {
  nodeId: "n0",
  description: "",
  entryName: "raw",
  directory: true,
  parent: null,
  reference: null,
  migration: null,
  children: [],
};

const DIRECTORY_NODE: StructureNodeView = {
  nodeId: "n1",
  description: "raw",
  entryName: "raw",
  directory: true,
  parent: null,
  reference: null,
  migration: { directory: "/opt/sop/raw", filter: [] },
  children: [],
};

const FILE_NODE: StructureNodeView = {
  nodeId: "n2",
  description: "",
  entryName: "a.sas7bdat",
  directory: false,
  parent: "n1",
  reference: "/opt/sop/raw/a.sas7bdat",
  migration: null,
  children: [],
};

function renderDrawer(
  props: Partial<ComponentProps<typeof NodeDrawer>> = {},
) {
  return render(
    <AegisThemeProvider>
      <AegisI18nProvider>
        <NodeDrawer
          open
          mode="create"
          node={null}
          parent={PARENT}
          onClose={() => {}}
          onSubmit={() => {}}
          pending={false}
          error={null}
          {...props}
        />
      </AegisI18nProvider>
    </AegisThemeProvider>,
  );
}

describe("isValidEntryName", () => {
  it("accepts a plain name", () => {
    expect(isValidEntryName("data")).toBe(true);
  });

  it("accepts a name with a space in it", () => {
    expect(isValidEntryName("raw data")).toBe(true);
  });

  it("rejects a blank name", () => {
    expect(isValidEntryName("   ")).toBe(false);
  });

  it("rejects a name carrying a path separator", () => {
    expect(isValidEntryName("raw/data")).toBe(false);
    expect(isValidEntryName("raw\\data")).toBe(false);
  });

  it("rejects a dot or dot-dot name", () => {
    expect(isValidEntryName(".")).toBe(false);
    expect(isValidEntryName("..")).toBe(false);
  });
});

describe("NodeDrawer", () => {
  it("starts with an empty name when adding", () => {
    renderDrawer();
    // A node carries only its own name — the parent supplies the
    // nesting — so there is nothing to prefill.
    expect(screen.getByLabelText(/^Entry name/)).toHaveValue("");
  });

  it("shows the reference field only for a file", async () => {
    renderDrawer();
    expect(screen.queryByLabelText("Reference path")).not.toBeInTheDocument();
    await userEvent.click(screen.getByLabelText("Folder"));
    expect(screen.getByLabelText("Reference path")).toBeInTheDocument();
    expect(
      screen.queryByLabelText("Migration source directory"),
    ).not.toBeInTheDocument();
  });

  it("shows the migration fields only for a directory", async () => {
    renderDrawer();
    expect(
      screen.getByLabelText("Migration source directory"),
    ).toBeInTheDocument();
    await userEvent.click(screen.getByLabelText("Folder"));
    expect(
      screen.queryByLabelText("Migration source directory"),
    ).not.toBeInTheDocument();
  });

  it("disables Save while the name is invalid", async () => {
    renderDrawer();
    const save = screen.getByRole("button", { name: "Create" });
    expect(save).toBeDisabled();
    await userEvent.type(screen.getByLabelText(/^Entry name/), "raw/data");
    expect(save).toBeDisabled();
    await userEvent.clear(screen.getByLabelText(/^Entry name/));
    await userEvent.type(screen.getByLabelText(/^Entry name/), "data");
    expect(save).toBeEnabled();
  });

  it("sends the name and the parent id when adding", async () => {
    const onSubmit = vi.fn();
    renderDrawer({ onSubmit });
    await userEvent.type(screen.getByLabelText(/^Entry name/), "data");
    await userEvent.click(screen.getByRole("button", { name: "Create" }));
    expect(onSubmit).toHaveBeenCalledWith(
      expect.objectContaining({ entryName: "data", parent: "n0" }),
    );
  });

  it("sends a null parent when adding at the root", async () => {
    const onSubmit = vi.fn();
    renderDrawer({ parent: null, onSubmit });
    await userEvent.type(screen.getByLabelText(/^Entry name/), "raw");
    await userEvent.click(screen.getByRole("button", { name: "Create" }));
    expect(onSubmit).toHaveBeenCalledWith(
      expect.objectContaining({ entryName: "raw", parent: null }),
    );
  });

  it("drops a blank migration filter", async () => {
    vi.mocked(open).mockResolvedValue("/opt/sop");
    const onSubmit = vi.fn();
    renderDrawer({ onSubmit });
    await userEvent.click(screen.getByRole("button", { name: /browse/i }));
    await userEvent.click(screen.getByRole("button", { name: "Add filter" }));
    await userEvent.type(
      screen.getAllByLabelText("Migration filter")[0],
      "*.sas7bdat",
    );
    // A second, blank filter row.
    await userEvent.click(screen.getByRole("button", { name: "Add filter" }));
    await userEvent.type(screen.getByLabelText(/^Entry name/), "raw");
    await userEvent.click(screen.getByRole("button", { name: "Create" }));
    expect(onSubmit).toHaveBeenCalledWith(
      expect.objectContaining({
        migration: { directory: "/opt/sop", filter: ["*.sas7bdat"] },
      }),
    );
  });

  it("shows the node's current values when editing", () => {
    renderDrawer({
      mode: "edit",
      node: {
        nodeId: "n1",
        description: "raw data",
        entryName: "data",
        directory: true,
        parent: "n0",
        reference: null,
        migration: null,
        children: [],
      },
      parent: null,
    });
    expect(screen.getByLabelText(/^Entry name/)).toHaveValue("data");
    expect(screen.getByLabelText("Description")).toHaveValue("raw data");
  });

  it("picks the migration source with a directory dialog", async () => {
    vi.mocked(open).mockResolvedValue("/opt/sop/raw2");
    renderDrawer({ mode: "edit", node: DIRECTORY_NODE });

    await userEvent.click(screen.getByRole("button", { name: /browse/i }));

    expect(open).toHaveBeenCalledWith({ directory: true, multiple: false });
    expect(screen.getByLabelText(/migration source directory/i)).toHaveValue(
      "/opt/sop/raw2",
    );
  });

  it("submits the picked migration source", async () => {
    vi.mocked(open).mockResolvedValue("/opt/sop/raw2");
    const onSubmit = vi.fn();
    renderDrawer({ mode: "edit", node: DIRECTORY_NODE, onSubmit });

    await userEvent.click(screen.getByRole("button", { name: /browse/i }));
    await userEvent.click(screen.getByRole("button", { name: "Save" }));

    expect(onSubmit).toHaveBeenCalledWith(
      expect.objectContaining({
        migration: { directory: "/opt/sop/raw2", filter: [] },
      }),
    );
  });

  it("picks the reference path with a file dialog", async () => {
    vi.mocked(open).mockResolvedValue("/opt/sop/raw/b.sas7bdat");
    renderDrawer({ mode: "edit", node: FILE_NODE });

    await userEvent.click(screen.getByRole("button", { name: /browse/i }));

    expect(open).toHaveBeenCalledWith({ directory: false, multiple: false });
    expect(screen.getByLabelText(/reference path/i)).toHaveValue(
      "/opt/sop/raw/b.sas7bdat",
    );
  });

  it("submits the picked reference path", async () => {
    vi.mocked(open).mockResolvedValue("/opt/sop/raw/b.sas7bdat");
    const onSubmit = vi.fn();
    renderDrawer({ mode: "edit", node: FILE_NODE, onSubmit });

    await userEvent.click(screen.getByRole("button", { name: /browse/i }));
    await userEvent.click(screen.getByRole("button", { name: "Save" }));

    expect(onSubmit).toHaveBeenCalledWith(
      expect.objectContaining({
        reference: "/opt/sop/raw/b.sas7bdat",
      }),
    );
  });

  it("keeps the reference path when the dialog is cancelled", async () => {
    vi.mocked(open).mockResolvedValue(null);
    const onSubmit = vi.fn();
    renderDrawer({ mode: "edit", node: FILE_NODE, onSubmit });

    await userEvent.click(screen.getByRole("button", { name: /browse/i }));
    await userEvent.click(screen.getByRole("button", { name: "Save" }));

    expect(onSubmit).toHaveBeenCalledWith(
      expect.objectContaining({
        reference: "/opt/sop/raw/a.sas7bdat",
      }),
    );
  });
});

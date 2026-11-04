import "@testing-library/jest-dom/vitest";
import { afterEach, describe, expect, it, vi } from "vitest";
import { cleanup, render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import type { ComponentProps } from "react";
import { AegisI18nProvider } from "@aegis/ui/i18n";
import { AegisThemeProvider } from "@aegis/ui/theme";

import type { StructureNodeView } from "../../../shared/api";
import {
  NodeDrawer,
  isValidEntryName,
} from "../../../features/templates/components/NodeDrawer";

afterEach(() => cleanup());

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
    const onSubmit = vi.fn();
    renderDrawer({ onSubmit });
    await userEvent.type(
      screen.getByLabelText("Migration source directory"),
      "/opt/sop",
    );
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
});

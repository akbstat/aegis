import "@testing-library/jest-dom/vitest";
import { afterEach, describe, expect, it, vi } from "vitest";
import { cleanup, render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import type { ComponentProps } from "react";
import { AegisI18nProvider } from "@aegis/ui/i18n";
import { AegisThemeProvider } from "@aegis/ui/theme";

import type { StructureNodeView } from "../../../shared/api";
import { StructureNodeRow } from "../../../features/templates/components/StructureNodeRow";

afterEach(() => cleanup());

const DIRECTORY: StructureNodeView = {
  nodeId: "n1",
  description: "Statistical programming raw data",
  entryName: "data",
  directory: true,
  parent: "n0",
  reference: null,
  migration: {
    directory: "/opt/sop/raw",
    filter: ["*.sas7bdat", "*.xpt"],
  },
  children: [],
};

const FILE: StructureNodeView = {
  nodeId: "n2",
  description: "",
  // The name differs from the reference's file name on purpose, so the
  // two are not mistaken for each other in the row.
  entryName: "b.sas7bdat",
  directory: false,
  parent: "n1",
  reference: "/opt/sop/raw/a.sas7bdat",
  migration: null,
  children: [],
};

function renderRow(
  node: StructureNodeView,
  props: Partial<ComponentProps<typeof StructureNodeRow>> = {},
) {
  return render(
    <AegisThemeProvider>
      <AegisI18nProvider>
        <StructureNodeRow
          node={node}
          canMutate={false}
          onAdd={() => {}}
          onEdit={() => {}}
          onDelete={() => {}}
          {...props}
        />
      </AegisI18nProvider>
    </AegisThemeProvider>,
  );
}

describe("StructureNodeRow", () => {
  it("shows the node's own name", () => {
    renderRow(DIRECTORY);
    expect(screen.getByText("data")).toBeInTheDocument();
  });

  it("does not show the description inline", () => {
    renderRow(DIRECTORY);
    expect(
      screen.queryByText("Statistical programming raw data"),
    ).not.toBeInTheDocument();
  });

  it("shows the description as a tooltip when the row is hovered", async () => {
    renderRow(DIRECTORY);
    await userEvent.hover(screen.getByText("data"));
    const tooltip = await screen.findByRole("tooltip");
    expect(tooltip).toHaveTextContent("Statistical programming raw data");
  });

  it("marks a directory with a folder icon and a file with a file icon", () => {
    const { unmount } = renderRow(DIRECTORY);
    expect(screen.getByTestId("FolderIcon")).toBeInTheDocument();
    unmount();

    renderRow(FILE);
    expect(screen.getByTestId("InsertDriveFileIcon")).toBeInTheDocument();
  });

  it("shows only the file name of the reference path", () => {
    renderRow(FILE);
    expect(screen.getByText("a.sas7bdat")).toBeInTheDocument();
    expect(
      screen.queryByText("/opt/sop/raw/a.sas7bdat"),
    ).not.toBeInTheDocument();
  });

  it("shows the complete reference path in a tooltip", async () => {
    renderRow(FILE);
    await userEvent.hover(screen.getByText("a.sas7bdat"));
    const tooltip = await screen.findByRole("tooltip");
    expect(tooltip).toHaveTextContent("/opt/sop/raw/a.sas7bdat");
  });

  it("shows the migration source as a file name with the whole path in a tooltip", async () => {
    renderRow(DIRECTORY);
    expect(screen.getByText("raw")).toBeInTheDocument();
    expect(screen.queryByText("/opt/sop/raw")).not.toBeInTheDocument();
    await userEvent.hover(screen.getByText("raw"));
    const tooltip = await screen.findByRole("tooltip");
    expect(tooltip).toHaveTextContent("/opt/sop/raw");
  });

  it("shows the migration filters as chips", () => {
    renderRow(DIRECTORY);
    const chips = document.querySelectorAll(".MuiChip-root");
    expect(chips).toHaveLength(2);
    expect(chips[0]).toHaveTextContent("*.sas7bdat");
    expect(chips[1]).toHaveTextContent("*.xpt");
  });

  it("shows the given label and the extra full path", () => {
    renderRow(DIRECTORY, { label: "data", secondaryPath: "raw/data" });
    expect(screen.getByText("data")).toBeInTheDocument();
    expect(screen.getByText("raw/data")).toBeInTheDocument();
  });

  it("hides the action menu from a read-only user", () => {
    renderRow(DIRECTORY, { canMutate: false });
    expect(
      screen.queryByRole("button", { name: /node actions/i }),
    ).not.toBeInTheDocument();
  });

  it("offers add, edit and delete to an admin", async () => {
    const onAdd = vi.fn();
    renderRow(DIRECTORY, { canMutate: true, onAdd });
    await userEvent.click(screen.getByRole("button", { name: /node actions/i }));
    expect(screen.getByText("Add node")).toBeInTheDocument();
    expect(screen.getByText("Edit node")).toBeInTheDocument();
    expect(screen.getByText("Delete node")).toBeInTheDocument();
    await userEvent.click(screen.getByText("Add node"));
    expect(onAdd).toHaveBeenCalledWith(DIRECTORY);
  });
});

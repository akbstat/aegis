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
  path: "raw/data",
  directory: true,
  parent: "n0",
  reference: null,
  migration: { directory: "/opt/sop/raw", filter: ["*.sas7bdat"] },
  children: [],
};

const FILE: StructureNodeView = {
  nodeId: "n2",
  description: "",
  path: "raw/data/a.sas7bdat",
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
  it("shows the node path", () => {
    renderRow(DIRECTORY);
    expect(screen.getByText("raw/data")).toBeInTheDocument();
  });

  it("shows the description, the migration source and the filters", () => {
    renderRow(DIRECTORY);
    expect(
      screen.getByText("Statistical programming raw data"),
    ).toBeInTheDocument();
    expect(screen.getByText("Migration")).toBeInTheDocument();
    expect(screen.getByText("/opt/sop/raw")).toBeInTheDocument();
    expect(screen.getByText("*.sas7bdat")).toBeInTheDocument();
  });

  it("omits the empty description instead of rendering a blank line", () => {
    renderRow(FILE);
    expect(screen.getByText("raw/data/a.sas7bdat")).toBeInTheDocument();
    expect(screen.queryByText("Migration")).not.toBeInTheDocument();
  });

  it("shows the reference path of a file", () => {
    renderRow(FILE);
    expect(screen.getByText("/opt/sop/raw/a.sas7bdat")).toBeInTheDocument();
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

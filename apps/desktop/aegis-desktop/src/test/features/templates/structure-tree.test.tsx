import "@testing-library/jest-dom/vitest";
import { afterEach, describe, expect, it } from "vitest";
import { cleanup, render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import type { ComponentProps } from "react";
import { AegisI18nProvider } from "@aegis/ui/i18n";
import { AegisThemeProvider } from "@aegis/ui/theme";

import type { StructureNodeView } from "../../../shared/api";
import { StructureTree } from "../../../features/templates/components/StructureTree";

afterEach(() => cleanup());

function dir(
  nodeId: string,
  entryName: string,
  children: StructureNodeView[] = [],
  description = "",
): StructureNodeView {
  return {
    nodeId,
    description,
    entryName,
    directory: true,
    parent: null,
    reference: null,
    migration: null,
    children,
  };
}

function file(
  nodeId: string,
  entryName: string,
  description = "",
): StructureNodeView {
  return {
    nodeId,
    description,
    entryName,
    directory: false,
    parent: null,
    reference: "/abs/path",
    migration: null,
    children: [],
  };
}

/**
 * A three-deep tree, so that a directory's own name and its full path
 * are two different strings and a query on one cannot match the other.
 */
const NODES: StructureNodeView[] = [
  dir("d0", "root", [
    dir("d1", "raw", [dir("d2", "data", [file("f1", "alpha.txt")])]),
  ]),
];

function renderTree(
  props: Partial<ComponentProps<typeof StructureTree>> = {},
) {
  return render(
    <AegisThemeProvider>
      <AegisI18nProvider>
        <StructureTree
          nodes={NODES}
          canMutate={false}
          canAddRoot={false}
          loading={false}
          onAdd={() => {}}
          onEdit={() => {}}
          onDelete={() => {}}
          {...props}
        />
      </AegisI18nProvider>
    </AegisThemeProvider>,
  );
}

async function showFlatView() {
  await userEvent.click(screen.getByRole("button", { name: /flat/i }));
}

function filterField() {
  return screen.getByRole("textbox", { name: "Filter folders" });
}

describe("StructureTree — flat view filter", () => {
  it("shows the filter field only in the flat view", async () => {
    renderTree();

    expect(
      screen.queryByRole("textbox", { name: "Filter folders" }),
    ).not.toBeInTheDocument();

    await showFlatView();

    expect(filterField()).toBeInTheDocument();
  });

  it("keeps only the directories matching the keyword", async () => {
    renderTree();
    await showFlatView();

    await userEvent.type(filterField(), "data");

    expect(screen.getByText("data")).toBeInTheDocument();
    expect(screen.queryByText("raw")).not.toBeInTheDocument();
  });

  it("keeps a directory whose file matches the keyword", async () => {
    renderTree();
    await showFlatView();

    await userEvent.type(filterField(), "alpha");

    expect(screen.getByText("data")).toBeInTheDocument();
    expect(screen.getByText("alpha.txt")).toBeInTheDocument();
    expect(screen.queryByText("raw")).not.toBeInTheDocument();
  });

  it("says so when nothing matches the keyword", async () => {
    renderTree();
    await showFlatView();

    await userEvent.type(filterField(), "zzz");

    expect(screen.queryByText("raw")).not.toBeInTheDocument();
    expect(screen.getByText(/no folders match/i)).toBeInTheDocument();
  });

  it("restores every directory when the filter is cleared", async () => {
    renderTree();
    await showFlatView();

    await userEvent.type(filterField(), "data");
    expect(screen.queryByText("raw")).not.toBeInTheDocument();

    await userEvent.click(
      screen.getByRole("button", { name: /clear filter/i }),
    );

    expect(screen.getByText("raw")).toBeInTheDocument();
    expect(screen.getByText("data")).toBeInTheDocument();
  });

  it("expands a directory again once the filter is cleared", async () => {
    renderTree();
    await showFlatView();

    await userEvent.type(filterField(), "data");
    await userEvent.click(
      screen.getByRole("button", { name: /clear filter/i }),
    );

    // The flat view shows every entry expanded, so a directory that
    // comes back must not stay collapsed.
    // In the flat view a directory's children are its files, so
    // "data" — the entry holding alpha.txt — is the one that expands.
    const data = screen.getByRole("treeitem", { name: /^data/ });
    expect(data.querySelector("[data-expanded]")).toHaveAttribute(
      "data-expanded",
      "",
    );
  });
});

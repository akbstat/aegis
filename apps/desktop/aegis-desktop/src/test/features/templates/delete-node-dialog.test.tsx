import "@testing-library/jest-dom/vitest";
import { afterEach, describe, expect, it, vi } from "vitest";
import { cleanup, render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import type { ComponentProps } from "react";
import { AegisI18nProvider } from "@aegis/ui/i18n";
import { AegisThemeProvider } from "@aegis/ui/theme";

import type { StructureNodeView } from "../../../shared/api";
import { DeleteNodeDialog } from "../../../features/templates/components/DeleteNodeDialog";

afterEach(() => cleanup());

const DIRECTORY: StructureNodeView = {
  nodeId: "n1",
  description: "",
  entryName: "data",
  directory: true,
  parent: "n0",
  reference: null,
  migration: null,
  children: [],
};

function renderDialog(
  props: Partial<ComponentProps<typeof DeleteNodeDialog>> = {},
) {
  return render(
    <AegisThemeProvider>
      <AegisI18nProvider>
        <DeleteNodeDialog
          open
          node={DIRECTORY}
          onClose={() => {}}
          onConfirm={() => {}}
          pending={false}
          error={null}
          {...props}
        />
      </AegisI18nProvider>
    </AegisThemeProvider>,
  );
}

describe("DeleteNodeDialog", () => {
  it("names the node and warns that its subtree goes too", () => {
    renderDialog();
    expect(
      screen.getByText(/Delete "data" and everything under it\?/),
    ).toBeInTheDocument();
  });

  it("confirms with the node", async () => {
    const onConfirm = vi.fn();
    renderDialog({ onConfirm });
    await userEvent.click(screen.getByRole("button", { name: "Confirm" }));
    expect(onConfirm).toHaveBeenCalledWith(DIRECTORY);
  });

  it("shows the error when the delete fails", () => {
    renderDialog({
      error: { kind: "http", status: 403, code: "forbidden", message: "nope" },
    });
    expect(screen.getByText(/nope/)).toBeInTheDocument();
  });
});

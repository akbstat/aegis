import "@testing-library/jest-dom/vitest";
import { afterEach, describe, expect, it, vi } from "vitest";
import { cleanup, render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import type { ComponentProps } from "react";
import { AegisI18nProvider } from "@aegis/ui/i18n";
import { AegisThemeProvider } from "@aegis/ui/theme";

import type { StructureSummaryView } from "../../../shared/api";
import { DeleteStructureDialog } from "../../../features/templates/components/DeleteStructureDialog";

afterEach(() => cleanup());

const STRUCTURE: StructureSummaryView = {
  id: 3,
  name: "Standard",
  level: "template",
  projectCode: null,
  templateId: null,
};

function renderDialog(
  props: Partial<ComponentProps<typeof DeleteStructureDialog>> = {},
) {
  return render(
    <AegisThemeProvider>
      <AegisI18nProvider>
        <DeleteStructureDialog
          open
          structure={STRUCTURE}
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

describe("DeleteStructureDialog", () => {
  it("names the structure and warns that its folders go too", () => {
    renderDialog();
    expect(
      screen.getByText(/Delete "Standard" and all of its folders\?/),
    ).toBeInTheDocument();
  });

  it("confirms with the structure", async () => {
    const onConfirm = vi.fn();
    renderDialog({ onConfirm });
    await userEvent.click(screen.getByRole("button", { name: "Confirm" }));
    expect(onConfirm).toHaveBeenCalledWith(STRUCTURE);
  });

  it("shows the error when the delete fails", () => {
    renderDialog({
      error: { kind: "http", status: 409, code: "template_in_use", message: "in use" },
    });
    expect(screen.getByText(/in use/)).toBeInTheDocument();
  });
});

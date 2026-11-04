import "@testing-library/jest-dom/vitest";
import { afterEach, describe, expect, it, vi } from "vitest";
import { cleanup, render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import type { ComponentProps } from "react";
import { AegisI18nProvider } from "@aegis/ui/i18n";
import { AegisThemeProvider } from "@aegis/ui/theme";

import type { StructureSummaryView } from "../../../shared/api";
import {
  StructureDrawer,
  isValidStructureName,
} from "../../../features/templates/components/StructureDrawer";

afterEach(() => cleanup());

const STRUCTURE: StructureSummaryView = {
  id: 3,
  name: "Standard",
  level: "template",
  projectCode: null,
  templateId: null,
};

function renderDrawer(
  props: Partial<ComponentProps<typeof StructureDrawer>> = {},
) {
  return render(
    <AegisThemeProvider>
      <AegisI18nProvider>
        <StructureDrawer
          open
          mode="create"
          structure={null}
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

describe("isValidStructureName", () => {
  it("accepts a non-blank name", () => {
    expect(isValidStructureName("Standard")).toBe(true);
  });

  it("rejects a blank name", () => {
    expect(isValidStructureName("   ")).toBe(false);
  });
});

describe("StructureDrawer", () => {
  it("starts empty in create mode", () => {
    renderDrawer();
    expect(screen.getByLabelText(/^Name/)).toHaveValue("");
    expect(
      screen.getByRole("heading", { name: /new template/i }),
    ).toBeInTheDocument();
  });

  it("shows the current name in rename mode", () => {
    renderDrawer({ mode: "rename", structure: STRUCTURE });
    expect(screen.getByLabelText(/^Name/)).toHaveValue("Standard");
    expect(
      screen.getByRole("heading", { name: /rename template/i }),
    ).toBeInTheDocument();
  });

  it("disables Save while the name is blank", async () => {
    renderDrawer();
    const save = screen.getByRole("button", { name: "Create" });
    expect(save).toBeDisabled();
    await userEvent.type(screen.getByLabelText(/^Name/), "Standard");
    expect(save).toBeEnabled();
  });

  it("submits the trimmed name", async () => {
    const onSubmit = vi.fn();
    renderDrawer({ onSubmit });
    await userEvent.type(screen.getByLabelText(/^Name/), "  Standard  ");
    await userEvent.click(screen.getByRole("button", { name: "Create" }));
    expect(onSubmit).toHaveBeenCalledWith("Standard");
  });

  it("shows the save error", () => {
    renderDrawer({
      error: { kind: "http", status: 400, code: "validation_failed", message: "bad" },
    });
    expect(screen.getByText(/bad/)).toBeInTheDocument();
  });
});

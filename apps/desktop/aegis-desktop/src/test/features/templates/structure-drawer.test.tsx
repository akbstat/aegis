import "@testing-library/jest-dom/vitest";
import { afterEach, describe, expect, it, vi } from "vitest";
import { cleanup, render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import type { ComponentProps } from "react";
import { AegisI18nProvider } from "@aegis/ui/i18n";
import { AegisThemeProvider } from "@aegis/ui/theme";

vi.mock("@tauri-apps/plugin-dialog", () => ({ open: vi.fn() }));

import { open } from "@tauri-apps/plugin-dialog";

import type { StructureSummaryView } from "../../../shared/api";
import {
  StructureDrawer,
  isValidStructureName,
} from "../../../features/templates/components/StructureDrawer";

afterEach(() => {
  cleanup();
  vi.mocked(open).mockReset();
});

const STRUCTURE: StructureSummaryView = {
  id: 3,
  name: "Standard",
  rootpath: "/abs/root",
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
    expect(screen.getByLabelText(/^Root path/)).toHaveValue("");
    expect(
      screen.getByRole("heading", { name: /new template/i }),
    ).toBeInTheDocument();
  });

  it("shows the current name and root path in update mode", () => {
    renderDrawer({ mode: "update", structure: STRUCTURE });
    expect(screen.getByLabelText(/^Name/)).toHaveValue("Standard");
    expect(screen.getByLabelText(/^Root path/)).toHaveValue("/abs/root");
    expect(
      screen.getByRole("heading", { name: /update template/i }),
    ).toBeInTheDocument();
  });

  it("disables Create while the name is blank", async () => {
    renderDrawer();
    const create = screen.getByRole("button", { name: "Create" });
    expect(create).toBeDisabled();
    await userEvent.type(screen.getByLabelText(/^Name/), "Standard");
    expect(create).toBeDisabled();
  });

  it("picks a root path with a directory dialog", async () => {
    vi.mocked(open).mockResolvedValue("/abs/root");
    renderDrawer();
    await userEvent.click(screen.getByRole("button", { name: /browse/i }));
    expect(open).toHaveBeenCalledWith({ directory: true, multiple: false });
    expect(screen.getByLabelText(/^Root path/)).toHaveValue("/abs/root");
  });

  it("enables Create once both the name and the root path are set", async () => {
    vi.mocked(open).mockResolvedValue("/abs/root");
    renderDrawer();
    await userEvent.type(screen.getByLabelText(/^Name/), "Standard");
    expect(screen.getByRole("button", { name: "Create" })).toBeDisabled();
    await userEvent.click(screen.getByRole("button", { name: /browse/i }));
    expect(screen.getByRole("button", { name: "Create" })).toBeEnabled();
  });

  it("submits the trimmed name and the picked root path", async () => {
    vi.mocked(open).mockResolvedValue("/abs/root");
    const onSubmit = vi.fn();
    renderDrawer({ onSubmit });
    await userEvent.type(screen.getByLabelText(/^Name/), "  Standard  ");
    await userEvent.click(screen.getByRole("button", { name: /browse/i }));
    await userEvent.click(screen.getByRole("button", { name: "Create" }));
    expect(onSubmit).toHaveBeenCalledWith({
      name: "Standard",
      rootpath: "/abs/root",
    });
  });

  it("sends no root path when the stored one is left alone", async () => {
    const onSubmit = vi.fn();
    renderDrawer({ mode: "update", structure: STRUCTURE, onSubmit });
    await userEvent.click(screen.getByRole("button", { name: "Save" }));
    expect(onSubmit).toHaveBeenCalledWith({
      name: "Standard",
      rootpath: null,
    });
  });

  it("sends the new root path when it is re-picked", async () => {
    vi.mocked(open).mockResolvedValue("/other/root");
    const onSubmit = vi.fn();
    renderDrawer({ mode: "update", structure: STRUCTURE, onSubmit });
    await userEvent.click(screen.getByRole("button", { name: /browse/i }));
    await userEvent.click(screen.getByRole("button", { name: "Save" }));
    expect(onSubmit).toHaveBeenCalledWith({
      name: "Standard",
      rootpath: "/other/root",
    });
  });

  it("shows the save error", () => {
    renderDrawer({
      error: { kind: "http", status: 400, code: "validation_failed", message: "bad" },
    });
    expect(screen.getByText(/bad/)).toBeInTheDocument();
  });
});

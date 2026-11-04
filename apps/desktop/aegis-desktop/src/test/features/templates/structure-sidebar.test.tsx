import "@testing-library/jest-dom/vitest";
import { afterEach, describe, expect, it, vi } from "vitest";
import { cleanup, render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import type { ComponentProps } from "react";
import { AegisI18nProvider } from "@aegis/ui/i18n";
import { AegisThemeProvider } from "@aegis/ui/theme";

import type { StructureSummaryView } from "../../../shared/api";
import { StructureSidebar } from "../../../features/templates/components/StructureSidebar";

afterEach(() => cleanup());

const STRUCTURES: StructureSummaryView[] = [
  {
    id: 3,
    name: "Standard",
    level: "template",
    projectCode: null,
    templateId: null,
  },
];

function renderSidebar(
  props: Partial<ComponentProps<typeof StructureSidebar>> = {},
) {
  return render(
    <AegisThemeProvider>
      <AegisI18nProvider>
        <StructureSidebar
          structures={STRUCTURES}
          selectedId={3}
          onSelect={() => {}}
          onCreate={() => {}}
          onRename={() => {}}
          onDelete={() => {}}
          canMutate={false}
          loading={false}
          {...props}
        />
      </AegisI18nProvider>
    </AegisThemeProvider>,
  );
}

describe("StructureSidebar", () => {
  it("lists the structures by name", () => {
    renderSidebar();
    expect(screen.getByText("Standard")).toBeInTheDocument();
  });

  it("hides the admin actions from a read-only user", () => {
    renderSidebar();
    expect(
      screen.queryByRole("button", { name: /add template/i }),
    ).not.toBeInTheDocument();
    expect(
      screen.queryByRole("button", { name: /rename template/i }),
    ).not.toBeInTheDocument();
    expect(
      screen.queryByRole("button", { name: /delete template/i }),
    ).not.toBeInTheDocument();
  });

  it("offers add, rename and delete to an admin", async () => {
    const onCreate = vi.fn();
    const onRename = vi.fn();
    const onDelete = vi.fn();
    renderSidebar({ canMutate: true, onCreate, onRename, onDelete });

    await userEvent.click(
      screen.getByRole("button", { name: /add template/i }),
    );
    expect(onCreate).toHaveBeenCalled();

    await userEvent.click(
      screen.getByRole("button", { name: /rename template/i }),
    );
    expect(onRename).toHaveBeenCalledWith(STRUCTURES[0]);

    await userEvent.click(
      screen.getByRole("button", { name: /delete template/i }),
    );
    expect(onDelete).toHaveBeenCalledWith(STRUCTURES[0]);
  });

  it("does not select the structure when an admin action is clicked", async () => {
    const onSelect = vi.fn();
    renderSidebar({ canMutate: true, onSelect });
    await userEvent.click(
      screen.getByRole("button", { name: /rename template/i }),
    );
    expect(onSelect).not.toHaveBeenCalled();
  });
});

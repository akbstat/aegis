import "@testing-library/jest-dom/vitest";
import { afterEach, describe, expect, it, vi } from "vitest";
import { cleanup, render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import type { ComponentProps } from "react";
import { AegisI18nProvider } from "@aegis/ui/i18n";
import { AegisThemeProvider } from "@aegis/ui/theme";

vi.mock("@tauri-apps/plugin-dialog", () => ({ open: vi.fn() }));

import { open } from "@tauri-apps/plugin-dialog";

import { PathPicker } from "../../../features/templates/components/PathPicker";

afterEach(() => {
  cleanup();
  vi.mocked(open).mockReset();
});

function renderPicker(
  props: Partial<ComponentProps<typeof PathPicker>> = {},
) {
  return render(
    <AegisThemeProvider>
      <AegisI18nProvider>
        <PathPicker
          label="Root path"
          value=""
          kind="directory"
          onChange={() => {}}
          {...props}
        />
      </AegisI18nProvider>
    </AegisThemeProvider>,
  );
}

describe("PathPicker", () => {
  it("opens a directory picker for a directory field", async () => {
    vi.mocked(open).mockResolvedValue("/abs/root");
    const onChange = vi.fn();
    renderPicker({ kind: "directory", onChange });

    await userEvent.click(screen.getByRole("button", { name: /browse/i }));

    expect(open).toHaveBeenCalledWith({
      directory: true,
      multiple: false,
    });
    expect(onChange).toHaveBeenCalledWith("/abs/root");
  });

  it("opens a file picker for a file field", async () => {
    vi.mocked(open).mockResolvedValue("/abs/root/a.sas7bdat");
    renderPicker({ kind: "file" });

    await userEvent.click(screen.getByRole("button", { name: /browse/i }));

    expect(open).toHaveBeenCalledWith({
      directory: false,
      multiple: false,
    });
  });

  it("keeps the current value when the dialog is cancelled", async () => {
    vi.mocked(open).mockResolvedValue(null);
    const onChange = vi.fn();
    renderPicker({ onChange });

    await userEvent.click(screen.getByRole("button", { name: /browse/i }));

    expect(onChange).not.toHaveBeenCalled();
  });

  it("shows the picked path", () => {
    renderPicker({ value: "/abs/root" });
    expect(screen.getByLabelText(/root path/i)).toHaveValue("/abs/root");
  });
});

import "@testing-library/jest-dom/vitest";
import { afterEach, describe, expect, it, vi } from "vitest";
import { cleanup, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));

import { renderWithQueryClient } from "../../helpers/render-with-query-client";
import { mockCommands } from "../../helpers/tauri-mock";
import {
  useAddStructureNode,
  useDeleteStructureNode,
  useGetStructure,
  useListTemplateStructures,
  useUpdateStructureNode,
} from "../../../features/templates/data/structures";

afterEach(() => cleanup());

const STRUCTURE = {
  id: 3,
  name: "Standard",
  level: "template",
  projectCode: null,
  templateId: null,
  nodes: [],
};

const TEMPLATES = [
  {
    id: 3,
    name: "Standard",
    level: "template",
    projectCode: null,
    templateId: null,
  },
];

function Probe() {
  const list = useListTemplateStructures();
  const structure = useGetStructure(3);
  const add = useAddStructureNode();
  const update = useUpdateStructureNode();
  const del = useDeleteStructureNode();
  return (
    <div>
      <span data-testid="list-status">{list.status}</span>
      <span data-testid="list-count">{list.data?.length ?? -1}</span>
      <span data-testid="structure-status">{structure.status}</span>
      <span data-testid="structure-name">{structure.data?.name ?? ""}</span>
      <button
        data-testid="add"
        onClick={() =>
          add.mutate({
            structureId: 3,
            input: {
              entryName: "raw",
              description: "",
              directory: true,
              parent: null,
              reference: null,
              migration: null,
            },
          })
        }
      >
        add
      </button>
      <button
        data-testid="update"
        onClick={() =>
          update.mutate({
            structureId: 3,
            nodeId: "n1",
            input: {
              entryName: "raw",
              description: "d",
              directory: true,
              parent: null,
              reference: null,
              migration: null,
            },
          })
        }
      >
        update
      </button>
      <button
        data-testid="delete"
        onClick={() => del.mutate({ structureId: 3, nodeId: "n1" })}
      >
        delete
      </button>
    </div>
  );
}

describe("templates data hooks", () => {
  it("lists the template structures", async () => {
    mockCommands({
      list_structures: () => TEMPLATES,
      get_structure: () => STRUCTURE,
    });
    renderWithQueryClient(<Probe />);
    expect(await screen.findByText("1")).toBeInTheDocument();
  });

  it("loads a structure by id", async () => {
    mockCommands({
      list_structures: () => TEMPLATES,
      get_structure: () => STRUCTURE,
    });
    renderWithQueryClient(<Probe />);
    expect(await screen.findByText("Standard")).toBeInTheDocument();
  });

  it("refetches the structure after a node is added", async () => {
    const getStructure = vi.fn(() => STRUCTURE);
    mockCommands({
      list_structures: () => TEMPLATES,
      get_structure: getStructure,
      add_structure_node: () => ({ nodeId: "n9" }),
    });
    renderWithQueryClient(<Probe />);
    await screen.findByText("Standard");
    const before = getStructure.mock.calls.length;
    await userEvent.click(screen.getByTestId("add"));
    await waitFor(() =>
      expect(getStructure.mock.calls.length).toBeGreaterThan(before),
    );
  });

  it("refetches the structure after a node is updated", async () => {
    const getStructure = vi.fn(() => STRUCTURE);
    mockCommands({
      list_structures: () => TEMPLATES,
      get_structure: getStructure,
      update_structure_node: () => ({ nodeId: "n1" }),
    });
    renderWithQueryClient(<Probe />);
    await screen.findByText("Standard");
    const before = getStructure.mock.calls.length;
    await userEvent.click(screen.getByTestId("update"));
    await waitFor(() =>
      expect(getStructure.mock.calls.length).toBeGreaterThan(before),
    );
  });

  it("refetches the structure after a node is deleted", async () => {
    const getStructure = vi.fn(() => STRUCTURE);
    mockCommands({
      list_structures: () => TEMPLATES,
      get_structure: getStructure,
      delete_structure_node: () => undefined,
    });
    renderWithQueryClient(<Probe />);
    await screen.findByText("Standard");
    const before = getStructure.mock.calls.length;
    await userEvent.click(screen.getByTestId("delete"));
    await waitFor(() =>
      expect(getStructure.mock.calls.length).toBeGreaterThan(before),
    );
  });
});

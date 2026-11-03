import { describe, expect, it, vi } from "vitest";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));

import { api } from "../../../shared/api";
import { mockCommands, mockInvoke } from "../../helpers/tauri-mock";

describe("structures api", () => {
  it("invokes list_structures with the level", async () => {
    mockCommands({ list_structures: () => [] });
    await api.listStructures("template");
    expect(mockInvoke).toHaveBeenCalledWith("list_structures", {
      level: "template",
    });
  });

  it("invokes get_structure with the id", async () => {
    mockCommands({ get_structure: () => ({}) });
    await api.getStructure(3);
    expect(mockInvoke).toHaveBeenCalledWith("get_structure", { id: 3 });
  });

  it("invokes add_structure_node with camelCase arguments", async () => {
    mockCommands({ add_structure_node: () => ({}) });
    await api.addStructureNode(3, {
      path: "raw",
      description: "d",
      directory: true,
      parent: null,
      reference: null,
      migration: null,
    });
    expect(mockInvoke).toHaveBeenCalledWith("add_structure_node", {
      structureId: 3,
      path: "raw",
      description: "d",
      directory: true,
      parent: null,
      reference: null,
      migration: null,
    });
  });

  it("invokes update_structure_node with the node id", async () => {
    mockCommands({ update_structure_node: () => ({}) });
    await api.updateStructureNode(3, "n1", {
      path: "raw",
      description: "d",
      directory: true,
      parent: null,
      reference: null,
      migration: null,
    });
    expect(mockInvoke).toHaveBeenCalledWith(
      "update_structure_node",
      expect.objectContaining({ structureId: 3, nodeId: "n1" }),
    );
  });

  it("invokes delete_structure_node with the structure and node ids", async () => {
    mockCommands({ delete_structure_node: () => undefined });
    await api.deleteStructureNode(3, "n1");
    expect(mockInvoke).toHaveBeenCalledWith("delete_structure_node", {
      structureId: 3,
      nodeId: "n1",
    });
  });
});

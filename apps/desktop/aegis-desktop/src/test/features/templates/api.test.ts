import { describe, expect, it, vi } from "vitest";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));

import { api } from "../../../shared/api";
import { mockCommands, mockInvoke } from "../../helpers/tauri-mock";

/**
 * The invoke payload keys each command expects.
 *
 * Tauri v2 derives them from the Rust commands' Rust parameter names:
 * `tauri-macros` lower-camel-cases each parameter, and `tauri`'s
 * resolver looks that key up in the payload with no whole-payload
 * fallback. The Rust side keeps its own copy of this list, checked
 * against the command signatures by
 * `commands::structures::tests::invoke_keys_match_the_documented_shape`.
 */
const EXPECTED_INVOKE_KEYS: Record<string, string[]> = {
  list_structures: ["level"],
  get_structure: ["id"],
  create_structure: ["body"],
  update_structure: ["id", "body"],
  delete_structure: ["id"],
  add_structure_node: ["structureId", "body"],
  update_structure_node: ["structureId", "nodeId", "body"],
  delete_structure_node: ["structureId", "nodeId"],
};

describe("structures api", () => {
  it("sends exactly the invoke keys the Rust commands expect", async () => {
    // Tauri v2 resolves each command argument by the lower-camel-case of
    // the Rust parameter name and looks the key up in the payload with
    // no whole-payload fallback. A wrapper that flattens a struct
    // parameter instead of nesting it under that parameter's name is
    // rejected at runtime with "missing required key body".
    const nodeInput = {
      entryName: "raw",
      description: "d",
      directory: true,
      parent: null,
      reference: null,
      migration: null,
    };
    const cases: Array<[string, () => Promise<unknown>]> = [
      ["list_structures", () => api.listStructures("template")],
      ["get_structure", () => api.getStructure(3)],
      ["add_structure_node", () => api.addStructureNode(3, nodeInput)],
      [
        "update_structure_node",
        () => api.updateStructureNode(3, "n1", nodeInput),
      ],
      ["delete_structure_node", () => api.deleteStructureNode(3, "n1")],
      [
        "create_structure",
        () =>
          api.createStructure({
            name: "Standard",
            rootpath: "/abs/root",
            level: "template",
            projectCode: null,
            templateId: null,
          }),
      ],
      [
        "update_structure",
        () => api.updateStructure(3, { name: "Renamed", rootpath: "/abs/root" }),
      ],
      ["delete_structure", () => api.deleteStructure(3)],
    ];

    mockCommands({
      list_structures: () => [],
      get_structure: () => ({}),
      add_structure_node: () => ({}),
      update_structure_node: () => ({}),
      delete_structure_node: () => undefined,
      create_structure: () => ({}),
      update_structure: () => ({}),
      delete_structure: () => undefined,
    });

    for (const [command, call] of cases) {
      mockInvoke.mockClear();
      await call();
      const payload = mockInvoke.mock.calls[0][1] as Record<string, unknown>;
      expect(Object.keys(payload).sort(), command).toEqual(
        [...EXPECTED_INVOKE_KEYS[command]].sort(),
      );
    }
  });

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
      entryName: "raw",
      description: "d",
      directory: true,
      parent: null,
      reference: null,
      migration: null,
    });
    expect(mockInvoke).toHaveBeenCalledWith("add_structure_node", {
      structureId: 3,
      body: {
        entryName: "raw",
        description: "d",
        directory: true,
        parent: null,
        reference: null,
        migration: null,
      },
    });
  });

  it("invokes update_structure_node with the node id", async () => {
    mockCommands({ update_structure_node: () => ({}) });
    await api.updateStructureNode(3, "n1", {
      entryName: "raw",
      description: "d",
      directory: true,
      parent: null,
      reference: null,
      migration: null,
    });
    expect(mockInvoke).toHaveBeenCalledWith(
      "update_structure_node",
      expect.objectContaining({
        structureId: 3,
        nodeId: "n1",
        body: expect.objectContaining({ entryName: "raw" }),
      }),
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

  it("invokes create_structure with the root path in the body", async () => {
    mockCommands({ create_structure: () => ({}) });
    await api.createStructure({
      name: "Standard",
      rootpath: "/abs/root",
      level: "template",
      projectCode: null,
      templateId: null,
    });
    expect(mockInvoke).toHaveBeenCalledWith("create_structure", {
      body: {
        name: "Standard",
        rootpath: "/abs/root",
        level: "template",
        projectCode: null,
        templateId: null,
      },
    });
  });

  it("invokes update_structure with the root path in the body", async () => {
    mockCommands({ update_structure: () => ({}) });
    await api.updateStructure(3, {
      name: "Renamed",
      rootpath: "/abs/root",
    });
    expect(mockInvoke).toHaveBeenCalledWith("update_structure", {
      id: 3,
      body: { name: "Renamed", rootpath: "/abs/root" },
    });
  });
});

import { describe, expect, it } from "vitest";

import type { StructureNodeView } from "../../../shared/api";
import {
  flattenStructure,
  nodeName,
} from "../../../features/templates/data/tree";

function dir(
  nodeId: string,
  path: string,
  children: StructureNodeView[] = [],
): StructureNodeView {
  return {
    nodeId,
    description: "",
    path,
    directory: true,
    parent: null,
    reference: null,
    migration: null,
    children,
  };
}

function file(nodeId: string, path: string): StructureNodeView {
  return {
    nodeId,
    description: "",
    path,
    directory: false,
    parent: null,
    reference: "/abs/path",
    migration: null,
    children: [],
  };
}

describe("flattenStructure", () => {
  it("promotes every directory to a top-level entry", () => {
    const entries = flattenStructure([
      dir("d1", "raw", [dir("d2", "raw/data")]),
    ]);
    expect(entries.map((e) => e.node.nodeId)).toEqual(["d1", "d2"]);
  });

  it("does not nest a sub-directory under its parent", () => {
    const entries = flattenStructure([
      dir("d1", "raw", [dir("d2", "raw/data")]),
    ]);
    const raw = entries.find((e) => e.node.nodeId === "d1")!;
    expect(raw.files).toEqual([]);
    expect(raw.node.children.map((c) => c.nodeId)).toEqual(["d2"]);
  });

  it("collects the files directly inside each directory", () => {
    const entries = flattenStructure([
      dir("d1", "raw", [
        file("f1", "raw/b.txt"),
        dir("d2", "raw/data", [file("f2", "raw/data/a.sas7bdat")]),
      ]),
    ]);
    const raw = entries.find((e) => e.node.nodeId === "d1")!;
    const data = entries.find((e) => e.node.nodeId === "d2")!;
    expect(raw.files.map((f) => f.nodeId)).toEqual(["f1"]);
    expect(data.files.map((f) => f.nodeId)).toEqual(["f2"]);
  });

  it("gives a root-level file its own top-level entry", () => {
    const entries = flattenStructure([file("f1", "README.md")]);
    expect(entries).toHaveLength(1);
    expect(entries[0].node.nodeId).toBe("f1");
    expect(entries[0].files).toEqual([]);
  });

  it("names an entry by the last segment of its path and keeps the full path", () => {
    const entries = flattenStructure([
      dir("d1", "raw", [dir("d2", "raw/data")]),
    ]);
    const data = entries.find((e) => e.node.nodeId === "d2")!;
    expect(data.name).toBe("data");
    expect(data.fullPath).toBe("raw/data");
  });

  it("returns an empty list for an empty tree", () => {
    expect(flattenStructure([])).toEqual([]);
  });
});

describe("nodeName", () => {
  it("returns the last path segment", () => {
    expect(nodeName("raw/data")).toBe("data");
  });

  it("returns the whole path when it has no separator", () => {
    expect(nodeName("raw")).toBe("raw");
  });
});

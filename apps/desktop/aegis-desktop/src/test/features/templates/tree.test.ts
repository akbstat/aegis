import { describe, expect, it } from "vitest";

import type { StructureNodeView } from "../../../shared/api";
import {
  buildFullPaths,
  flattenStructure,
} from "../../../features/templates/data/tree";

function dir(
  nodeId: string,
  entryName: string,
  children: StructureNodeView[] = [],
): StructureNodeView {
  return {
    nodeId,
    description: "",
    entryName,
    directory: true,
    parent: null,
    reference: null,
    migration: null,
    children,
  };
}

function file(nodeId: string, entryName: string): StructureNodeView {
  return {
    nodeId,
    description: "",
    entryName,
    directory: false,
    parent: null,
    reference: "/abs/path",
    migration: null,
    children: [],
  };
}

describe("buildFullPaths", () => {
  it("gives a root node its own name", () => {
    const paths = buildFullPaths([dir("d1", "raw")]);
    expect(paths.get("d1")).toBe("raw");
  });

  it("joins the ancestor names with a slash", () => {
    const paths = buildFullPaths([
      dir("d1", "raw", [dir("d2", "data", [file("f1", "a.sas7bdat")])]),
    ]);
    expect(paths.get("d1")).toBe("raw");
    expect(paths.get("d2")).toBe("raw/data");
    expect(paths.get("f1")).toBe("raw/data/a.sas7bdat");
  });

  it("gives a root-level file its own name", () => {
    const paths = buildFullPaths([file("f1", "README.md")]);
    expect(paths.get("f1")).toBe("README.md");
  });

  it("returns an empty map for an empty tree", () => {
    expect(buildFullPaths([]).size).toBe(0);
  });
});

describe("flattenStructure", () => {
  it("promotes every directory to a top-level entry", () => {
    const entries = flattenStructure([
      dir("d1", "raw", [dir("d2", "data")]),
    ]);
    expect(entries.map((e) => e.node.nodeId)).toEqual(["d1", "d2"]);
  });

  it("does not nest a sub-directory under its parent", () => {
    const entries = flattenStructure([
      dir("d1", "raw", [dir("d2", "data")]),
    ]);
    const raw = entries.find((e) => e.node.nodeId === "d1")!;
    expect(raw.files).toEqual([]);
    expect(raw.node.children.map((c) => c.nodeId)).toEqual(["d2"]);
  });

  it("collects the files directly inside each directory", () => {
    const entries = flattenStructure([
      dir("d1", "raw", [
        file("f1", "b.txt"),
        dir("d2", "data", [file("f2", "a.sas7bdat")]),
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

  it("names an entry by its own name and computes the full path", () => {
    const entries = flattenStructure([
      dir("d1", "raw", [dir("d2", "data")]),
    ]);
    const data = entries.find((e) => e.node.nodeId === "d2")!;
    expect(data.name).toBe("data");
    // The full path is derived from the ancestors — the server no longer
    // stores one.
    expect(data.fullPath).toBe("raw/data");
  });

  it("returns an empty list for an empty tree", () => {
    expect(flattenStructure([])).toEqual([]);
  });
});

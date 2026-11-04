import { describe, expect, it } from "vitest";

import type { StructureNodeView } from "../../../shared/api";
import {
  buildFullPaths,
  filterFlatEntries,
  flattenStructure,
} from "../../../features/templates/data/tree";

function dir(
  nodeId: string,
  entryName: string,
  children: StructureNodeView[] = [],
  description = "",
): StructureNodeView {
  return {
    nodeId,
    description,
    entryName,
    directory: true,
    parent: null,
    reference: null,
    migration: null,
    children,
  };
}

function file(
  nodeId: string,
  entryName: string,
  description = "",
): StructureNodeView {
  return {
    nodeId,
    description,
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

describe("filterFlatEntries", () => {
  it("keeps a directory whose name contains the keyword", () => {
    const entries = flattenStructure([
      dir("d1", "raw"),
      dir("d2", "data"),
    ]);

    const kept = filterFlatEntries(entries, "raw");

    expect(kept.map((e) => e.node.nodeId)).toEqual(["d1"]);
  });

  it("drops a directory that does not match the keyword", () => {
    const entries = flattenStructure([dir("d1", "data")]);

    expect(filterFlatEntries(entries, "raw")).toEqual([]);
  });

  it("keeps a directory whose description contains the keyword", () => {
    const entries = flattenStructure([dir("d1", "raw", [], "the raw zone")]);

    const kept = filterFlatEntries(entries, "raw zone");

    expect(kept.map((e) => e.node.nodeId)).toEqual(["d1"]);
  });

  it("keeps a directory when a file inside it matches the keyword", () => {
    const entries = flattenStructure([
      dir("d1", "raw", [file("f1", "a.sas7bdat")]),
      dir("d2", "data"),
    ]);

    const kept = filterFlatEntries(entries, "sas7bdat");

    expect(kept.map((e) => e.node.nodeId)).toEqual(["d1"]);
  });

  it("keeps a directory when a file inside it matches on its description", () => {
    const entries = flattenStructure([
      dir("d1", "raw", [file("f1", "a.sas7bdat", "the subject data")]),
    ]);

    const kept = filterFlatEntries(entries, "subject data");

    expect(kept.map((e) => e.node.nodeId)).toEqual(["d1"]);
  });

  it("keeps every file of a surviving directory", () => {
    const entries = flattenStructure([
      dir("d1", "raw", [file("f1", "a.sas7bdat"), file("f2", "b.txt")]),
    ]);

    const kept = filterFlatEntries(entries, "sas7bdat");

    // The keyword decides which directories survive; a surviving
    // directory keeps its whole file list.
    expect(kept[0].files.map((f) => f.nodeId)).toEqual(["f1", "f2"]);
  });

  it("matches the keyword regardless of case", () => {
    const entries = flattenStructure([dir("d1", "Raw")]);

    const kept = filterFlatEntries(entries, "raw");

    expect(kept.map((e) => e.node.nodeId)).toEqual(["d1"]);
  });

  it("treats a blank keyword as no filter at all", () => {
    const entries = flattenStructure([dir("d1", "raw"), dir("d2", "data")]);

    expect(filterFlatEntries(entries, "   ")).toEqual(entries);
  });

  it("keeps a root-level file that matches on its own name", () => {
    const entries = flattenStructure([file("f1", "README.md"), dir("d2", "raw")]);

    const kept = filterFlatEntries(entries, "readme");

    expect(kept.map((e) => e.node.nodeId)).toEqual(["f1"]);
  });
});

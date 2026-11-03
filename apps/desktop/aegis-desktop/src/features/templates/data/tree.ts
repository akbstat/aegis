import type { StructureNodeView } from "../../../shared/api";

/**
 * One top-level entry of the flat view.
 *
 * A directory becomes an entry of its own; the files sitting directly
 * inside it become that entry's leaves. A sub-directory never nests
 * under its parent — it gets its own entry.
 */
export interface FlatEntry {
  /** The directory node, or a root-level file. */
  node: StructureNodeView;
  /** The node's own name — the last segment of its path. */
  name: string;
  /** The node's full path relative to the structure root. */
  fullPath: string;
  /** The files directly inside the directory. Empty for a file entry. */
  files: StructureNodeView[];
}

/** The last `/`-separated segment of a path. */
export function nodeName(path: string): string {
  return path.split("/").pop() ?? path;
}

/**
 * Flatten a structure tree into one entry per directory.
 *
 * Every directory, at any depth, becomes a top-level entry; a
 * sub-directory is never nested under its parent. The files directly
 * inside a directory become that directory's leaves. A file with no
 * parent directory has nothing to sit under, so it gets its own entry.
 */
export function flattenStructure(nodes: StructureNodeView[]): FlatEntry[] {
  const entries: FlatEntry[] = [];

  const walk = (siblings: StructureNodeView[], atRoot: boolean) => {
    for (const node of siblings) {
      if (node.directory) {
        entries.push({
          node,
          name: nodeName(node.path),
          fullPath: node.path,
          files: node.children.filter((c) => !c.directory),
        });
        walk(
          node.children.filter((c) => c.directory),
          false,
        );
      } else if (atRoot) {
        entries.push({
          node,
          name: nodeName(node.path),
          fullPath: node.path,
          files: [],
        });
      }
    }
  };

  walk(nodes, true);
  return entries;
}

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
  /** The node's own name — the folder or file name it carries. */
  name: string;
  /** The path from the structure root, computed from the ancestors. */
  fullPath: string;
  /** The files directly inside the directory. Empty for a file entry. */
  files: StructureNodeView[];
}

/**
 * The full path of every node in the tree, keyed by node id.
 *
 * A node carries only its own name — the nesting comes from `parent`,
 * and the server stores no path — so the path relative to the
 * structure root is derived here, by joining the ancestor names.
 */
export function buildFullPaths(
  nodes: StructureNodeView[],
): Map<string, string> {
  const paths = new Map<string, string>();
  const walk = (siblings: StructureNodeView[], prefix: string) => {
    for (const node of siblings) {
      const fullPath = prefix
        ? `${prefix}/${node.entryName}`
        : node.entryName;
      paths.set(node.nodeId, fullPath);
      walk(node.children, fullPath);
    }
  };
  walk(nodes, "");
  return paths;
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
  const paths = buildFullPaths(nodes);
  const entries: FlatEntry[] = [];

  const walk = (siblings: StructureNodeView[], atRoot: boolean) => {
    for (const node of siblings) {
      const fullPath = paths.get(node.nodeId) ?? node.entryName;
      if (node.directory) {
        entries.push({
          node,
          name: node.entryName,
          fullPath,
          files: node.children.filter((c) => !c.directory),
        });
        walk(
          node.children.filter((c) => c.directory),
          false,
        );
      } else if (atRoot) {
        entries.push({
          node,
          name: node.entryName,
          fullPath,
          files: [],
        });
      }
    }
  };

  walk(nodes, true);
  return entries;
}

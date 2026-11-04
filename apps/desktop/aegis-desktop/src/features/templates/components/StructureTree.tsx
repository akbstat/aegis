import { useState } from "react";
import {
  Box,
  Button,
  Stack,
  ToggleButton,
  ToggleButtonGroup,
  Typography,
} from "@aegis/ui/mui";
import { Add as AddIcon } from "@aegis/ui/icons";
import { useI18n } from "@aegis/ui/i18n";
import { SimpleTreeView, TreeItem } from "@aegis/ui/tree-view";

import type { StructureNodeView } from "../../../shared/api";
import { flattenStructure } from "../data/tree";
import { StructureNodeRow } from "./StructureNodeRow";

export type StructureTreeViewMode = "complete" | "flat";

export interface StructureTreeProps {
  nodes: StructureNodeView[];
  canMutate: boolean;
  /** A root node can only be added to a selected structure. */
  canAddRoot: boolean;
  /** Whether the structure is still being fetched. */
  loading: boolean;
  onAdd: (node: StructureNodeView | null) => void;
  onEdit: (node: StructureNodeView) => void;
  onDelete: (node: StructureNodeView) => void;
}

interface TreeProps {
  nodes: StructureNodeView[];
  canMutate: boolean;
  onAdd: (node: StructureNodeView) => void;
  onEdit: (node: StructureNodeView) => void;
  onDelete: (node: StructureNodeView) => void;
}

/**
 * The ids of every directory in the tree.
 *
 * The tree renders fully expanded: a folder-structure page is read to
 * see the whole layout, so a collapsed root would hide most of it.
 */
function directoryIds(nodes: StructureNodeView[]): string[] {
  const ids: string[] = [];
  const walk = (list: StructureNodeView[]) => {
    for (const node of list) {
      if (node.directory) {
        ids.push(node.nodeId);
        walk(node.children);
      }
    }
  };
  walk(nodes);
  return ids;
}

/** The Complete tree — the API's tree, nested by `parent`. */
function CompleteTree(props: TreeProps) {
  const renderNodes = (nodes: StructureNodeView[]) =>
    nodes.map((node) => (
      <TreeItem
        key={node.nodeId}
        itemId={node.nodeId}
        label={
          <StructureNodeRow
            node={node}
            canMutate={props.canMutate}
            onAdd={props.onAdd}
            onEdit={props.onEdit}
            onDelete={props.onDelete}
          />
        }
      >
        {node.children.length > 0 ? renderNodes(node.children) : null}
      </TreeItem>
    ));

  return (
    <SimpleTreeView defaultExpandedItems={directoryIds(props.nodes)}>
      {renderNodes(props.nodes)}
    </SimpleTreeView>
  );
}

/** The Flat tree — one row per directory, with its files beneath it. */
function FlatTree(props: TreeProps) {
  const entries = flattenStructure(props.nodes);

  return (
    // The flat view exists to show a directory's files as its leaves,
    // so every entry starts expanded.
    <SimpleTreeView defaultExpandedItems={entries.map((e) => e.node.nodeId)}>
      {entries.map((entry) => (
        <TreeItem
          key={entry.node.nodeId}
          itemId={entry.node.nodeId}
          label={
            <StructureNodeRow
              node={entry.node}
              canMutate={props.canMutate}
              onAdd={props.onAdd}
              onEdit={props.onEdit}
              onDelete={props.onDelete}
              label={entry.name}
              secondaryPath={entry.fullPath}
            />
          }
        >
          {entry.files.map((file) => (
            <TreeItem
              key={file.nodeId}
              itemId={file.nodeId}
              label={
                <StructureNodeRow
                  node={file}
                  canMutate={props.canMutate}
                  onAdd={props.onAdd}
                  onEdit={props.onEdit}
                  onDelete={props.onDelete}
                />
              }
            />
          ))}
        </TreeItem>
      ))}
    </SimpleTreeView>
  );
}

export function StructureTree({
  nodes,
  canMutate,
  canAddRoot,
  loading,
  onAdd,
  onEdit,
  onDelete,
}: StructureTreeProps) {
  const { t } = useI18n();
  const [mode, setMode] = useState<StructureTreeViewMode>("complete");

  const shared: TreeProps = { nodes, canMutate, onAdd, onEdit, onDelete };

  return (
    <Box
      sx={{
        flexGrow: 1,
        minWidth: 0,
        minHeight: 0,
        height: "100%",
        display: "flex",
        flexDirection: "column",
        overflow: "hidden",
      }}
    >
      <Stack
        direction="row"
        spacing={2}
        sx={{ px: 3, pt: 3, pb: 2, flexShrink: 0, alignItems: "center" }}
      >
        <ToggleButtonGroup
          size="small"
          exclusive
          value={mode}
          onChange={(_e, next) => {
            if (next) setMode(next as StructureTreeViewMode);
          }}
        >
          <ToggleButton value="complete">
            {t("templates.structure.view.complete")}
          </ToggleButton>
          <ToggleButton value="flat">
            {t("templates.structure.view.flat")}
          </ToggleButton>
        </ToggleButtonGroup>

        {canMutate && canAddRoot && (
          <Button
            size="small"
            startIcon={<AddIcon />}
            onClick={() => onAdd(null)}
          >
            {t("templates.node.addRoot")}
          </Button>
        )}
      </Stack>

      <Box sx={{ flexGrow: 1, minHeight: 0, overflow: "auto", px: 3, pb: 3 }}>
        {nodes.length === 0 ? (
          <Typography variant="body2" color="text.secondary">
            {loading ? "…" : t("templates.structure.noNodes")}
          </Typography>
        ) : mode === "complete" ? (
          <CompleteTree {...shared} />
        ) : (
          <FlatTree {...shared} />
        )}
      </Box>
    </Box>
  );
}

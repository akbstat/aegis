import { Box, Chip, Stack, Tooltip, Typography } from "@aegis/ui/mui";
import {
  Folder as FolderIcon,
  InsertDriveFile as InsertDriveFileIcon,
} from "@aegis/ui/icons";
import { useI18n } from "@aegis/ui/i18n";

import type { StructureNodeView } from "../../../shared/api";
import { NodeActionMenu } from "./NodeActionMenu";

export interface StructureNodeRowProps {
  node: StructureNodeView;
  canMutate: boolean;
  onAdd: (node: StructureNodeView) => void;
  onEdit: (node: StructureNodeView) => void;
  onDelete: (node: StructureNodeView) => void;
  /** Overrides the label; the flat view shows the node's own name. */
  label?: string;
  /** The flat view's extra full-path line. */
  secondaryPath?: string;
}

/**
 * The last segment of a path — the file or folder name.
 *
 * The reference and the migration source are absolute paths; only the
 * name is shown inline, with the whole path in the tooltip.
 */
export function basename(path: string): string {
  return path.split(/[\\/]/).pop() ?? path;
}

/**
 * One node's row: a folder or file icon, then the node's own name, then
 * the reference and the migration.
 *
 * The description is not shown inline — it is the row's tooltip, so a
 * long description cannot push the name out of view. The reference and
 * the migration source are shown by name only, with the whole path in
 * their tooltip; the migration filters are chips.
 */
export function StructureNodeRow({
  node,
  canMutate,
  onAdd,
  onEdit,
  onDelete,
  label,
  secondaryPath,
}: StructureNodeRowProps) {
  const { t } = useI18n();

  return (
    <Stack direction="row" spacing={1} sx={{ alignItems: "center" }}>
      {node.directory ? (
        <FolderIcon fontSize="small" color="primary" />
      ) : (
        <InsertDriveFileIcon fontSize="small" color="action" />
      )}

      <Box sx={{ flexGrow: 1, minWidth: 0 }}>
        <Tooltip title={node.description} describeChild placement="top">
          <Typography variant="body2" noWrap sx={{ fontWeight: 600 }}>
            {label ?? node.entryName}
          </Typography>
        </Tooltip>

        {secondaryPath && (
          <Typography variant="caption" color="text.secondary" noWrap>
            {secondaryPath}
          </Typography>
        )}

        {node.reference && (
          <Tooltip title={node.reference} describeChild placement="top">
            <Typography variant="caption" color="text.secondary" noWrap>
              {basename(node.reference)}
            </Typography>
          </Tooltip>
        )}

        {node.migration && (
          <>
            <Typography variant="caption" color="text.secondary" noWrap>
              {t("templates.node.field.migration")}
            </Typography>
            <Tooltip
              title={node.migration.directory}
              describeChild
              placement="top"
            >
              <Typography variant="caption" color="text.secondary" noWrap>
                {basename(node.migration.directory)}
              </Typography>
            </Tooltip>
            {node.migration.filter.length > 0 && (
              <Box sx={{ display: "flex", flexWrap: "wrap", gap: 0.5 }}>
                {node.migration.filter.map((filter) => (
                  <Chip
                    key={filter}
                    size="small"
                    label={filter}
                    sx={{ height: 18, fontSize: 11 }}
                  />
                ))}
              </Box>
            )}
          </>
        )}
      </Box>

      {canMutate && (
        <NodeActionMenu
          node={node}
          onAdd={onAdd}
          onEdit={onEdit}
          onDelete={onDelete}
        />
      )}
    </Stack>
  );
}

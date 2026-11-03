import { Box, Stack, Typography } from "@aegis/ui/mui";
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
 * One node's row: the path as the label, then the description, the
 * reference and the migration as secondary text. An empty description
 * is skipped rather than rendered as a blank line.
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
    <Stack direction="row" spacing={1} alignItems="flex-start">
      <Box sx={{ flexGrow: 1, minWidth: 0 }}>
        <Typography variant="body2" fontWeight={600} noWrap>
          {label ?? node.path}
        </Typography>
        {secondaryPath && (
          <Typography variant="caption" color="text.secondary" noWrap>
            {secondaryPath}
          </Typography>
        )}
        {node.description && (
          <Typography variant="caption" color="text.secondary" noWrap>
            {node.description}
          </Typography>
        )}
        {node.reference && (
          <Typography variant="caption" color="text.secondary" noWrap>
            {node.reference}
          </Typography>
        )}
        {node.migration && (
          <>
            <Typography variant="caption" color="text.secondary" noWrap>
              {t("templates.node.field.migration")}
            </Typography>
            <Typography variant="caption" color="text.secondary" noWrap>
              {node.migration.directory}
            </Typography>
            {node.migration.filter.map((f) => (
              <Typography
                key={f}
                variant="caption"
                color="text.secondary"
                noWrap
              >
                {f}
              </Typography>
            ))}
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

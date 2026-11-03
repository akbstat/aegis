import { useState } from "react";
import { IconButton, Menu, MenuItem } from "@aegis/ui/mui";
import {
  Add as AddIcon,
  Delete as DeleteIcon,
  Edit as EditIcon,
  MoreHoriz as MoreHorizIcon,
} from "@aegis/ui/icons";
import { useI18n } from "@aegis/ui/i18n";

import type { StructureNodeView } from "../../../shared/api";

export interface NodeActionMenuProps {
  node: StructureNodeView;
  onAdd: (node: StructureNodeView) => void;
  onEdit: (node: StructureNodeView) => void;
  onDelete: (node: StructureNodeView) => void;
}

/**
 * The per-node admin menu. The spec asks for "a list of icon buttons";
 * each entry carries an action icon *and* its label, so the three
 * actions are readable rather than merely recognisable.
 */
export function NodeActionMenu({
  node,
  onAdd,
  onEdit,
  onDelete,
}: NodeActionMenuProps) {
  const { t } = useI18n();
  const [anchor, setAnchor] = useState<HTMLElement | null>(null);

  const close = () => setAnchor(null);

  return (
    <>
      <IconButton
        size="small"
        aria-label={t("templates.node.add")}
        onClick={(e) => setAnchor(e.currentTarget)}
      >
        <MoreHorizIcon fontSize="small" />
      </IconButton>
      <Menu anchorEl={anchor} open={anchor != null} onClose={close}>
        <MenuItem
          onClick={() => {
            close();
            onAdd(node);
          }}
        >
          <AddIcon fontSize="small" sx={{ mr: 1 }} />
          {t("templates.node.add")}
        </MenuItem>
        <MenuItem
          onClick={() => {
            close();
            onEdit(node);
          }}
        >
          <EditIcon fontSize="small" sx={{ mr: 1 }} />
          {t("templates.node.edit")}
        </MenuItem>
        <MenuItem
          onClick={() => {
            close();
            onDelete(node);
          }}
        >
          <DeleteIcon fontSize="small" sx={{ mr: 1 }} />
          {t("templates.node.delete")}
        </MenuItem>
      </Menu>
    </>
  );
}

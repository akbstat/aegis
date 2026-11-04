import {
  Box,
  IconButton,
  List,
  ListItemButton,
  ListItemText,
  Stack,
  Typography,
} from "@aegis/ui/mui";
import { Add as AddIcon, Delete as DeleteIcon, Edit as EditIcon } from "@aegis/ui/icons";
import { useI18n } from "@aegis/ui/i18n";

import type { StructureSummaryView } from "../../../shared/api";

export interface StructureSidebarProps {
  structures: StructureSummaryView[];
  selectedId: number | null;
  onSelect: (id: number) => void;
  /** Open the drawer for a new template structure. */
  onCreate: () => void;
  /** Open the drawer to rename this structure. */
  onRename: (structure: StructureSummaryView) => void;
  /** Open the confirm dialog to delete this structure. */
  onDelete: (structure: StructureSummaryView) => void;
  /** Whether the current user may write. Admin and root only. */
  canMutate: boolean;
  loading: boolean;
}

/** The template-level structures, listed down the right-hand side. */
export function StructureSidebar({
  structures,
  selectedId,
  onSelect,
  onCreate,
  onRename,
  onDelete,
  canMutate,
  loading,
}: StructureSidebarProps) {
  const { t } = useI18n();

  return (
    <Box
      data-testid="structure-sidebar"
      sx={{
        width: 260,
        height: "100%",
        flexShrink: 0,
        borderLeft: 1,
        borderColor: "divider",
        overflowY: "auto",
        p: 2,
      }}
    >
      <Stack
        direction="row"
        sx={{ mb: 1, alignItems: "center", justifyContent: "space-between" }}
      >
        <Typography variant="subtitle2">{t("templates.structure.list")}</Typography>
        {canMutate && (
          <IconButton
            size="small"
            aria-label={t("templates.structure.add")}
            onClick={onCreate}
          >
            <AddIcon fontSize="small" />
          </IconButton>
        )}
      </Stack>

      {loading && <Typography variant="body2">…</Typography>}

      {!loading && structures.length === 0 && (
        <Typography variant="body2" color="text.secondary">
          {t("templates.structure.noTemplates")}
        </Typography>
      )}

      <List disablePadding>
        {structures.map((structure) => (
          <ListItemButton
            key={structure.id}
            selected={structure.id === selectedId}
            onClick={() => onSelect(structure.id)}
          >
            <ListItemText primary={structure.name} />
            {canMutate && (
              <Stack direction="row" spacing={0}>
                <IconButton
                  size="small"
                  aria-label={t("templates.structure.rename")}
                  onClick={(e) => {
                    // The row itself selects the structure — an admin
                    // action must not also trigger that.
                    e.stopPropagation();
                    onRename(structure);
                  }}
                >
                  <EditIcon fontSize="small" />
                </IconButton>
                <IconButton
                  size="small"
                  aria-label={t("templates.structure.delete")}
                  onClick={(e) => {
                    e.stopPropagation();
                    onDelete(structure);
                  }}
                >
                  <DeleteIcon fontSize="small" />
                </IconButton>
              </Stack>
            )}
          </ListItemButton>
        ))}
      </List>
    </Box>
  );
}

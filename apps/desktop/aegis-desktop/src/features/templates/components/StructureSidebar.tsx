import {
  Box,
  List,
  ListItemButton,
  ListItemText,
  Typography,
} from "@aegis/ui/mui";
import { useI18n } from "@aegis/ui/i18n";

import type { StructureSummaryView } from "../../../shared/api";

export interface StructureSidebarProps {
  structures: StructureSummaryView[];
  selectedId: number | null;
  onSelect: (id: number) => void;
  loading: boolean;
}

/** The template-level structures, listed down the right-hand side. */
export function StructureSidebar({
  structures,
  selectedId,
  onSelect,
  loading,
}: StructureSidebarProps) {
  const { t } = useI18n();

  return (
    <Box
      data-testid="structure-sidebar"
      sx={{
        width: 260,
        flexShrink: 0,
        borderLeft: 1,
        borderColor: "divider",
        overflowY: "auto",
        p: 2,
      }}
    >
      <Typography variant="subtitle2" sx={{ mb: 1 }}>
        {t("templates.structure.list")}
      </Typography>

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
          </ListItemButton>
        ))}
      </List>
    </Box>
  );
}

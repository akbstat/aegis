import { useState } from "react";
import {
  Button,
  Dialog,
  DialogActions,
  DialogContent,
  DialogTitle,
  MenuItem,
  TextField,
  Typography,
} from "@aegis/ui/mui";
import { useI18n } from "@aegis/ui/i18n";

import { useListTemplateStructures } from "../../templates";
import type { ProjectBranchView } from "../../../shared/api";

export interface InitializeProjectFolderDialogProps {
  open: boolean;
  /** The branch the folder is being initialized for, when the dialog
   *  was opened from a branch row. */
  branch?: ProjectBranchView;
  onClose: () => void;
  /** Called with the chosen structure's name when the user confirms. */
  onInitialized: (structureName: string) => void;
}

/**
 * "Initialize project folder" dialog.
 *
 * There is no server endpoint for this yet — the dialog picks the
 * template structure the folder would be seeded from and reports the
 * choice, and the caller shows the success message. It deliberately
 * makes no request beyond reading the template structures.
 */
export function InitializeProjectFolderDialog({
  open,
  branch,
  onClose,
  onInitialized,
}: InitializeProjectFolderDialogProps) {
  const { t } = useI18n();
  const structures = useListTemplateStructures();
  const [structureId, setStructureId] = useState<number | null>(null);

  const selected = structures.data?.find((s) => s.id === structureId);

  function handleConfirm() {
    if (!selected) return;
    onInitialized(selected.name);
    onClose();
  }

  return (
    <Dialog open={open} onClose={onClose}>
      <DialogTitle>{t("project.branches.initialize.title")}</DialogTitle>
      <DialogContent>
        {branch && (
          <Typography variant="body2" sx={{ mb: 2 }}>
            {branch.branch}
          </Typography>
        )}
        <TextField
          select
          fullWidth
          size="small"
          label={t("project.branches.initialize.structure")}
          value={structureId ?? ""}
          onChange={(e) => {
            const raw = e.target.value;
            setStructureId(raw === "" ? null : Number(raw));
          }}
          helperText={
            structureId === null
              ? t("project.branches.initialize.required")
              : undefined
          }
          data-testid="initialize-folder-structure"
        >
          {(structures.data ?? []).map((s) => (
            <MenuItem key={s.id} value={s.id}>
              {s.name}
            </MenuItem>
          ))}
        </TextField>
      </DialogContent>
      <DialogActions>
        <Button onClick={onClose}>{t("common.cancel")}</Button>
        <Button
          variant="contained"
          disabled={!selected}
          onClick={handleConfirm}
          data-testid="initialize-folder-confirm"
        >
          {t("common.confirm")}
        </Button>
      </DialogActions>
    </Dialog>
  );
}

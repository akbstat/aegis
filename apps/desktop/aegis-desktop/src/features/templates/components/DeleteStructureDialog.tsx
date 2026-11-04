import {
  Button,
  Dialog,
  DialogActions,
  DialogContent,
  DialogContentText,
  DialogTitle,
} from "@aegis/ui/mui";
import { useI18n } from "@aegis/ui/i18n";

import { errorMessage } from "../../../shared/api/error";
import type { StructureSummaryView } from "../../../shared/api";

export interface DeleteStructureDialogProps {
  open: boolean;
  structure: StructureSummaryView | null;
  onClose: () => void;
  onConfirm: (structure: StructureSummaryView) => void;
  pending: boolean;
  error: unknown;
}

export function DeleteStructureDialog({
  open,
  structure,
  onClose,
  onConfirm,
  pending,
  error,
}: DeleteStructureDialogProps) {
  const { t } = useI18n();
  return (
    <Dialog open={open} onClose={onClose}>
      <DialogTitle>{t("templates.structure.delete")}</DialogTitle>
      <DialogContent>
        <DialogContentText>
          {structure
            ? t("templates.structure.deleteMessage", { name: structure.name })
            : t("templates.structure.delete")}
        </DialogContentText>
        {Boolean(error) && (
          <DialogContentText sx={{ mt: 2, color: "error.main" }}>
            {t("templates.structure.deleteFailed", {
              message: errorMessage(error),
            })}
          </DialogContentText>
        )}
      </DialogContent>
      <DialogActions>
        <Button onClick={onClose} disabled={pending}>
          {t("common.cancel")}
        </Button>
        <Button
          color="error"
          onClick={() => structure && onConfirm(structure)}
          disabled={pending || !structure}
        >
          {t("common.confirm")}
        </Button>
      </DialogActions>
    </Dialog>
  );
}

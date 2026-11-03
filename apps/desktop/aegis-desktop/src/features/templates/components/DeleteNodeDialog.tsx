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
import type { StructureNodeView } from "../../../shared/api";

export interface DeleteNodeDialogProps {
  open: boolean;
  node: StructureNodeView | null;
  onClose: () => void;
  onConfirm: (node: StructureNodeView) => void;
  pending: boolean;
  error: unknown;
}

export function DeleteNodeDialog({
  open,
  node,
  onClose,
  onConfirm,
  pending,
  error,
}: DeleteNodeDialogProps) {
  const { t } = useI18n();
  return (
    <Dialog open={open} onClose={onClose}>
      <DialogTitle>{t("templates.node.delete.title")}</DialogTitle>
      <DialogContent>
        <DialogContentText>
          {node
            ? t("templates.node.delete.message", { path: node.path })
            : t("templates.node.delete.title")}
        </DialogContentText>
        {Boolean(error) && (
          <DialogContentText sx={{ mt: 2, color: "error.main" }}>
            {t("templates.node.delete.failed", {
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
          onClick={() => node && onConfirm(node)}
          disabled={pending || !node}
        >
          {t("common.confirm")}
        </Button>
      </DialogActions>
    </Dialog>
  );
}

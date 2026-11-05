import { useEffect, useState } from "react";
import {
  Alert,
  Button,
  Dialog,
  DialogActions,
  DialogContent,
  DialogTitle,
  TextField,
} from "@aegis/ui/mui";
import { useI18n } from "@aegis/ui/i18n";

import { errorMessage } from "../../../shared/api/error";
import type { ApiError, ProjectBranchView } from "../../../shared/api";

export interface BranchNameDialogProps {
  open: boolean;
  mode: "create" | "edit";
  /** The branch being renamed, in `edit` mode. */
  branch?: ProjectBranchView;
  pending: boolean;
  error: ApiError | null;
  onClose: () => void;
  onSubmit: (name: string) => void;
}

/**
 * Create / rename dialog for a project branch.
 *
 * The dialog owns the name field; the caller owns the mutation and
 * decides when to close it. An error is rendered inside the dialog, so
 * the caller keeps it open on failure — the server's `duplicate_branch`
 * or `forbidden` message reaches the user where they clicked.
 */
export function BranchNameDialog({
  open,
  mode,
  branch,
  pending,
  error,
  onClose,
  onSubmit,
}: BranchNameDialogProps) {
  const { t } = useI18n();
  const [name, setName] = useState("");

  // Reset the field whenever the dialog opens or the target changes.
  // `branch` is a stable object while the dialog is open — the caller
  // keeps it in state — so this cannot fire on every render.
  useEffect(() => {
    if (!open) return;
    setName(branch?.branch ?? "");
  }, [open, mode, branch]);

  const trimmed = name.trim();
  const blank = trimmed.length === 0;
  // Only flag the field once something has been typed, so an untouched
  // empty field does not open red.
  const showRequired = name.length > 0 && blank;

  return (
    <Dialog open={open} onClose={onClose}>
      <DialogTitle>
        {mode === "create"
          ? t("project.branches.dialog.create.title")
          : t("project.branches.dialog.rename.title")}
      </DialogTitle>
      <DialogContent>
        <TextField
          autoFocus
          fullWidth
          size="small"
          label={t("project.branches.dialog.name.label")}
          value={name}
          onChange={(e) => setName(e.target.value)}
          error={showRequired}
          helperText={
            showRequired ? t("project.branches.dialog.name.required") : undefined
          }
        />
        {error && (
          <Alert severity="error" sx={{ mt: 2 }}>
            {errorMessage(error)}
          </Alert>
        )}
      </DialogContent>
      <DialogActions>
        <Button onClick={onClose}>{t("common.cancel")}</Button>
        <Button
          variant="contained"
          disabled={blank || pending}
          onClick={() => onSubmit(trimmed)}
        >
          {t("common.confirm")}
        </Button>
      </DialogActions>
    </Dialog>
  );
}

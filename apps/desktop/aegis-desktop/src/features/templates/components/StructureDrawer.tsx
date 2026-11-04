import { useEffect, useState } from "react";
import {
  Alert,
  Box,
  Button,
  Drawer,
  Stack,
  TextField,
  Typography,
} from "@aegis/ui/mui";
import { useI18n } from "@aegis/ui/i18n";

import { errorMessage } from "../../../shared/api/error";
import type { ApiError, StructureSummaryView } from "../../../shared/api";

export interface StructureDrawerProps {
  open: boolean;
  /** `create` for a new template, `rename` for an existing one. */
  mode: "create" | "rename";
  /** The structure being renamed. `null` in create mode. */
  structure: StructureSummaryView | null;
  onClose: () => void;
  onSubmit: (name: string) => void;
  pending: boolean;
  error: ApiError | null;
}

/**
 * A structure's name only has to be non-blank — the server rejects an
 * empty one with `EmptyName` and takes anything else.
 */
export function isValidStructureName(name: string): boolean {
  return name.trim() !== "";
}

export function StructureDrawer({
  open,
  mode,
  structure,
  onClose,
  onSubmit,
  pending,
  error,
}: StructureDrawerProps) {
  const { t } = useI18n();
  const [name, setName] = useState("");

  useEffect(() => {
    if (!open) return;
    setName(mode === "create" ? "" : (structure?.name ?? ""));
  }, [open, mode, structure]);

  const nameValid = isValidStructureName(name);

  return (
    <Drawer
      anchor="right"
      open={open}
      onClose={onClose}
      slotProps={{ paper: { sx: { width: 420 } } }}
    >
      <Box sx={{ p: 3, display: "flex", flexDirection: "column", gap: 2 }}>
        <Typography variant="h6">
          {mode === "create"
            ? t("templates.structure.createTitle")
            : t("templates.structure.rename")}
        </Typography>

        <Stack spacing={2}>
          <TextField
            size="small"
            label={t("templates.structure.name")}
            value={name}
            onChange={(e) => setName(e.target.value)}
            required
            autoFocus
          />
        </Stack>

        {error && (
          <Alert severity="error">
            {t("templates.structure.saveFailed", {
              message: errorMessage(error),
            })}
          </Alert>
        )}

        <Box sx={{ display: "flex", gap: 1, justifyContent: "flex-end" }}>
          <Button onClick={onClose} disabled={pending}>
            {t("common.cancel")}
          </Button>
          <Button
            variant="contained"
            onClick={() => onSubmit(name.trim())}
            disabled={!nameValid || pending}
          >
            {mode === "create" ? t("common.create") : t("common.save")}
          </Button>
        </Box>
      </Box>
    </Drawer>
  );
}

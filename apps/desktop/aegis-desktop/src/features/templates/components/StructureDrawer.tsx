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
import { PathPicker } from "./PathPicker";

export interface StructureDrawerProps {
  open: boolean;
  /** `create` for a new template, `update` for an existing one. */
  mode: "create" | "update";
  /** The structure being updated. `null` in create mode. */
  structure: StructureSummaryView | null;
  onClose: () => void;
  onSubmit: (value: StructureFormValue) => void;
  pending: boolean;
  error: ApiError | null;
}

/**
 * What the drawer hands back. `rootpath` is `null` when the stored root
 * path should be left as it is — the server keeps it in that case.
 */
export interface StructureFormValue {
  name: string;
  rootpath: string | null;
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
  const [rootpath, setRootpath] = useState("");

  useEffect(() => {
    if (!open) return;
    if (mode === "create") {
      setName("");
      setRootpath("");
      return;
    }
    setName(structure?.name ?? "");
    setRootpath(structure?.rootpath ?? "");
  }, [open, mode, structure]);

  const nameValid = isValidStructureName(name);
  // The server rejects a create without a root path. An update may keep
  // the stored one, so it needs no root path of its own.
  const rootpathValid = mode === "update" || rootpath.trim() !== "";

  function handleSubmit() {
    if (!nameValid || !rootpathValid) return;
    const trimmedRootpath = rootpath.trim();
    // In update mode an untouched — or emptied — root path means "keep
    // the stored one".
    const nextRootpath =
      mode === "create" || trimmedRootpath !== (structure?.rootpath ?? "")
        ? trimmedRootpath
        : null;
    onSubmit({ name: name.trim(), rootpath: nextRootpath });
  }

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
            : t("templates.structure.update")}
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
          <PathPicker
            label={t("templates.structure.rootpath")}
            value={rootpath}
            kind="directory"
            onChange={setRootpath}
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
            onClick={handleSubmit}
            disabled={!nameValid || !rootpathValid || pending}
          >
            {mode === "create" ? t("common.create") : t("common.save")}
          </Button>
        </Box>
      </Box>
    </Drawer>
  );
}

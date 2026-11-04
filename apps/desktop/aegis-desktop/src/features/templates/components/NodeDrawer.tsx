import { useEffect, useState } from "react";
import {
  Alert,
  Box,
  Button,
  Drawer,
  FormControlLabel,
  Stack,
  Switch,
  TextField,
  Typography,
} from "@aegis/ui/mui";
import { useI18n } from "@aegis/ui/i18n";

import { errorMessage } from "../../../shared/api/error";
import type {
  ApiError,
  MigrationView,
  StructureNodeInput,
  StructureNodeView,
} from "../../../shared/api";

export interface NodeDrawerProps {
  open: boolean;
  mode: "create" | "edit";
  /** The node being edited. `null` in create mode. */
  node: StructureNodeView | null;
  /** The parent the new node goes under. `null` means the root. */
  parent: StructureNodeView | null;
  onClose: () => void;
  onSubmit: (input: StructureNodeInput) => void;
  pending: boolean;
  error: ApiError | null;
}

/**
 * A node's name is well formed when it is non-blank, carries no path
 * separator, and is neither `.` nor `..` — the same rule the server's
 * `validate_name` enforces. The nesting comes from `parent`, so a name
 * is unique among its siblings, not across the structure.
 */
export function isValidEntryName(name: string): boolean {
  const trimmed = name.trim();
  if (trimmed === "") return false;
  if (trimmed.includes("/") || trimmed.includes("\\")) return false;
  if (trimmed === "." || trimmed === "..") return false;
  return true;
}

export function NodeDrawer({
  open,
  mode,
  node,
  parent,
  onClose,
  onSubmit,
  pending,
  error,
}: NodeDrawerProps) {
  const { t } = useI18n();

  const [entryName, setEntryName] = useState("");
  const [description, setDescription] = useState("");
  const [directory, setDirectory] = useState(true);
  const [reference, setReference] = useState("");
  const [migrationDirectory, setMigrationDirectory] = useState("");
  const [migrationFilters, setMigrationFilters] = useState<string[]>([]);

  useEffect(() => {
    if (!open) return;
    if (mode === "create") {
      // A node carries only its own name — the parent supplies the
      // nesting — so there is nothing to prefill but the name itself.
      setEntryName("");
      setDescription("");
      setDirectory(true);
      setReference("");
      setMigrationDirectory("");
      setMigrationFilters([]);
      return;
    }
    setEntryName(node?.entryName ?? "");
    setDescription(node?.description ?? "");
    setDirectory(node?.directory ?? true);
    setReference(node?.reference ?? "");
    setMigrationDirectory(node?.migration?.directory ?? "");
    setMigrationFilters(node?.migration?.filter ?? []);
  }, [open, mode, node, parent]);

  const nameValid = isValidEntryName(entryName);

  function handleSubmit() {
    if (!nameValid) return;
    const migration: MigrationView | null = directory
      ? migrationDirectory.trim() === ""
        ? null
        : {
            directory: migrationDirectory.trim(),
            // A filter row the admin left blank is dropped, the same way
            // the domain-model drawer drops an empty language row.
            filter: migrationFilters.filter((f) => f.trim() !== ""),
          }
      : null;
    onSubmit({
      entryName: entryName.trim(),
      description,
      directory,
      parent:
        mode === "create" ? (parent?.nodeId ?? null) : (node?.parent ?? null),
      reference: directory ? null : reference.trim() || null,
      migration,
    });
  }

  return (
    <Drawer
      anchor="right"
      open={open}
      onClose={onClose}
      slotProps={{ paper: { sx: { width: 480 } } }}
    >
      <Box sx={{ p: 3, display: "flex", flexDirection: "column", gap: 2 }}>
        <Typography variant="h6">
          {mode === "create"
            ? t("templates.node.drawer.addTitle")
            : t("templates.node.drawer.editTitle")}
        </Typography>

        <Stack spacing={2}>
          <TextField
            size="small"
            label={t("templates.node.field.entryName")}
            value={entryName}
            onChange={(e) => setEntryName(e.target.value)}
            required
            error={entryName !== "" && !nameValid}
            helperText={
              entryName !== "" && !nameValid
                ? t("templates.node.field.invalidEntryName")
                : undefined
            }
          />
          <TextField
            size="small"
            label={t("templates.node.field.description")}
            value={description}
            onChange={(e) => setDescription(e.target.value)}
          />
          <FormControlLabel
            control={
              <Switch
                checked={directory}
                onChange={(e) => setDirectory(e.target.checked)}
              />
            }
            label={t("templates.node.field.directory")}
          />
          {!directory && (
            <TextField
              size="small"
              label={t("templates.node.field.reference")}
              value={reference}
              onChange={(e) => setReference(e.target.value)}
            />
          )}
          {directory && (
            <>
              <TextField
                size="small"
                label={t("templates.node.field.migrationDirectory")}
                value={migrationDirectory}
                onChange={(e) => setMigrationDirectory(e.target.value)}
              />
              {migrationFilters.map((filter, index) => (
                <TextField
                  key={index}
                  size="small"
                  label={t("templates.node.field.migrationFilter")}
                  value={filter}
                  onChange={(e) =>
                    setMigrationFilters((filters) =>
                      filters.map((f, i) =>
                        i === index ? e.target.value : f,
                      ),
                    )
                  }
                />
              ))}
              <Button
                size="small"
                sx={{ alignSelf: "flex-start" }}
                onClick={() => setMigrationFilters((filters) => [...filters, ""])}
              >
                {t("templates.node.field.migrationFilterAdd")}
              </Button>
            </>
          )}
        </Stack>

        {error && (
          <Alert severity="error">
            {t("templates.node.saveFailed", { message: errorMessage(error) })}
          </Alert>
        )}

        <Box sx={{ display: "flex", gap: 1, justifyContent: "flex-end" }}>
          <Button onClick={onClose} disabled={pending}>
            {t("common.cancel")}
          </Button>
          <Button
            variant="contained"
            onClick={handleSubmit}
            disabled={!nameValid || pending}
          >
            {mode === "create" ? t("common.create") : t("common.save")}
          </Button>
        </Box>
      </Box>
    </Drawer>
  );
}

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
 * A relative path is well formed when it is non-blank, uses `/` as its
 * only separator, and has no empty, `.` or `..` segment — the same rule
 * the server's `validate_path` enforces.
 */
export function isValidNodePath(path: string): boolean {
  const trimmed = path.trim();
  if (trimmed === "") return false;
  if (trimmed.startsWith("/") || trimmed.endsWith("/")) return false;
  if (trimmed.includes("\\")) return false;
  return trimmed
    .split("/")
    .every((seg) => seg !== "" && seg !== "." && seg !== "..");
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

  const [path, setPath] = useState("");
  const [description, setDescription] = useState("");
  const [directory, setDirectory] = useState(true);
  const [reference, setReference] = useState("");
  const [migrationDirectory, setMigrationDirectory] = useState("");
  const [migrationFilters, setMigrationFilters] = useState<string[]>([]);

  useEffect(() => {
    if (!open) return;
    if (mode === "create") {
      setPath(parent ? `${parent.path}/` : "");
      setDescription("");
      setDirectory(true);
      setReference("");
      setMigrationDirectory("");
      setMigrationFilters([]);
      return;
    }
    setPath(node?.path ?? "");
    setDescription(node?.description ?? "");
    setDirectory(node?.directory ?? true);
    setReference(node?.reference ?? "");
    setMigrationDirectory(node?.migration?.directory ?? "");
    setMigrationFilters(node?.migration?.filter ?? []);
  }, [open, mode, node, parent]);

  const pathValid = isValidNodePath(path);

  function handleSubmit() {
    if (!pathValid) return;
    const migration: MigrationView | null = directory
      ? migrationDirectory.trim() === ""
        ? null
        : { directory: migrationDirectory.trim(), filter: migrationFilters }
      : null;
    onSubmit({
      path: path.trim(),
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
            label={t("templates.node.field.path")}
            value={path}
            onChange={(e) => setPath(e.target.value)}
            required
            error={path !== "" && !pathValid}
            helperText={
              path !== "" && !pathValid
                ? t("templates.node.field.invalidPath")
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
                {t("templates.node.field.migrationFilter")}
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
            disabled={!pathValid || pending}
          >
            {mode === "create" ? t("common.create") : t("common.save")}
          </Button>
        </Box>
      </Box>
    </Drawer>
  );
}

import { Button, Stack, TextField } from "@aegis/ui/mui";
import { useI18n } from "@aegis/ui/i18n";
import { open } from "@tauri-apps/plugin-dialog";

export interface PathPickerProps {
  /** The field's label. */
  label: string;
  /** The chosen path. Empty means nothing has been picked yet. */
  value: string;
  /** `directory` opens a folder picker; `file` opens a file picker. */
  kind: "directory" | "file";
  /** Called with the picked path. Not called when the dialog is cancelled. */
  onChange: (path: string) => void;
}

/**
 * A read-only path field with a button that opens the OS picker.
 *
 * The path is picked rather than typed: a folder structure's root path
 * and a node's migration source are directories on the machine that runs
 * the desktop app, so typing them by hand invites typos the server
 * cannot detect.
 */
export function PathPicker({
  label,
  value,
  kind,
  onChange,
}: PathPickerProps) {
  const { t } = useI18n();

  async function pick() {
    const picked = await open({
      directory: kind === "directory",
      multiple: false,
    });
    // The dialog answers `null` when the user cancels it.
    if (typeof picked === "string") onChange(picked);
  }

  return (
    <Stack direction="row" spacing={1} sx={{ alignItems: "flex-start" }}>
      <TextField
        size="small"
        label={label}
        value={value}
        slotProps={{ input: { readOnly: true } }}
        sx={{ flexGrow: 1 }}
      />
      <Button onClick={pick} sx={{ mt: 0.5 }} variant="outlined">
        {t("templates.path.browse")}
      </Button>
    </Stack>
  );
}

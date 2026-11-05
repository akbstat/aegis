import { useState } from "react";
import {
  Alert,
  IconButton,
  Snackbar,
  Switch,
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableRow,
  Typography,
} from "@aegis/ui/mui";
import { Add, Edit, FolderOpen } from "@aegis/ui/icons";
import { useI18n } from "@aegis/ui/i18n";

import { errorMessage } from "../../../shared/api/error";
import type { ProjectBranchView } from "../../../shared/api";
import {
  useActiveBranchId,
  useCreateProjectBranch,
  useProjectBranches,
  useSetActiveBranchId,
  useUpdateProjectBranch,
} from "../data/project-branches";
import { BranchNameDialog } from "./BranchNameDialog";
import { InitializeProjectFolderDialog } from "./InitializeProjectFolderDialog";

export interface ProjectBranchesSectionProps {
  projectCode: string;
  /** Whether the current user may create, rename and initialize
   *  branches — the project leaders. */
  isLeader: boolean;
}

type NameDialogState = {
  mode: "create" | "edit";
  branch?: ProjectBranchView;
};

/**
 * The project's branches: one row per branch, with a switch that makes
 * it the active one.
 *
 * The switch is available to everyone and writes the desktop's own
 * active-branch state — the server has no active branch to set, so no
 * request is made. Creating and renaming are leader-only and do talk
 * to the server.
 */
export function ProjectBranchesSection({
  projectCode,
  isLeader,
}: ProjectBranchesSectionProps) {
  const { t } = useI18n();
  const branches = useProjectBranches(projectCode);
  const { data: activeBranchId } = useActiveBranchId(projectCode);
  const setActiveBranchId = useSetActiveBranchId(projectCode);
  const createBranch = useCreateProjectBranch(projectCode);
  const updateBranch = useUpdateProjectBranch(projectCode);

  const [nameDialog, setNameDialog] = useState<NameDialogState | null>(null);
  const [initializeBranch, setInitializeBranch] = useState<ProjectBranchView | null>(
    null,
  );
  const [initialized, setInitialized] = useState(false);

  const mutationError =
    nameDialog?.mode === "create"
      ? createBranch.error
      : nameDialog?.mode === "edit"
        ? updateBranch.error
        : null;
  const mutationPending =
    nameDialog?.mode === "create"
      ? createBranch.isPending
      : nameDialog?.mode === "edit"
        ? updateBranch.isPending
        : false;

  function openCreate() {
    createBranch.reset();
    setNameDialog({ mode: "create" });
  }

  function openRename(branch: ProjectBranchView) {
    updateBranch.reset();
    setNameDialog({ mode: "edit", branch });
  }

  async function handleSubmit(name: string) {
    if (!nameDialog) return;
    try {
      if (nameDialog.mode === "create") {
        await createBranch.mutateAsync({ branch: name });
      } else if (nameDialog.branch) {
        await updateBranch.mutateAsync({
          id: nameDialog.branch.id,
          body: { branch: name },
        });
      }
      setNameDialog(null);
    } catch {
      // The error is rendered inside the dialog, which stays open.
    }
  }

  const rows = branches.data ?? [];

  return (
    <>
      {branches.isError && (
        <Alert severity="error">
          {t("project.branches.loadFailed", {
            message: errorMessage(branches.error),
          })}
        </Alert>
      )}

      {rows.length === 0 && !branches.isError ? (
        <Typography color="text.secondary">
          {t("project.branches.empty")}
        </Typography>
      ) : (
        <Table size="small" data-testid="project-branches-table">
          <TableHead>
            <TableRow>
              <TableCell sx={{ width: 72 }} />
              <TableCell>{t("project.branches.name.label")}</TableCell>
              {isLeader && (
                <TableCell sx={{ width: 96 }}>
                  <IconButton
                    aria-label={t("project.branches.create")}
                    data-testid="branch-create"
                    onClick={openCreate}
                  >
                    <Add />
                  </IconButton>
                </TableCell>
              )}
            </TableRow>
          </TableHead>
          <TableBody>
            {rows.map((branch) => (
              <TableRow key={branch.id}>
                <TableCell>
                  <Switch
                    checked={branch.id === activeBranchId}
                    onChange={(e) =>
                      setActiveBranchId.mutate(e.target.checked ? branch.id : null)
                    }
                    slotProps={{
                      input: {
                        "aria-label": `${t("workspace.branch.active")}: ${branch.branch}`,
                      },
                    }}
                  />
                </TableCell>
                <TableCell>{branch.branch}</TableCell>
                {isLeader && (
                  <TableCell>
                    <IconButton
                      aria-label={t("project.branches.rename")}
                      data-testid={`branch-rename-${branch.id}`}
                      onClick={() => openRename(branch)}
                    >
                      <Edit />
                    </IconButton>
                    <IconButton
                      aria-label={t("project.branches.initializeFolder")}
                      data-testid={`branch-initialize-${branch.id}`}
                      onClick={() => setInitializeBranch(branch)}
                    >
                      <FolderOpen />
                    </IconButton>
                  </TableCell>
                )}
              </TableRow>
            ))}
          </TableBody>
        </Table>
      )}

      <BranchNameDialog
        open={nameDialog !== null}
        mode={nameDialog?.mode ?? "create"}
        branch={nameDialog?.branch}
        pending={mutationPending}
        error={mutationError ?? null}
        onClose={() => setNameDialog(null)}
        onSubmit={(name) => void handleSubmit(name)}
      />

      {/* Mounted only while open, so the template structures are
          fetched when the user asks for them and not before. */}
      {initializeBranch && (
        <InitializeProjectFolderDialog
          open
          branch={initializeBranch}
          onClose={() => setInitializeBranch(null)}
          onInitialized={() => setInitialized(true)}
        />
      )}

      <Snackbar
        open={initialized}
        autoHideDuration={6000}
        onClose={() => setInitialized(false)}
        message={t("project.branches.initialize.success")}
      />
    </>
  );
}

import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";

import {
  api,
  type ApiError,
  type CreateProjectBranchInput,
  type ProjectBranchView,
  type UpdateProjectBranchInput,
} from "../../../shared/api";
import { queryKeys } from "../../../shared/query";
import { readActiveBranch, writeActiveBranch } from "../../settings";

/**
 * The project's branches, as the server knows them.
 *
 * A branch carries no `active` flag — the server dropped it from the
 * branch row, and which branch the desktop is working on is local
 * state (see `useActiveBranchId`).
 */
export function useProjectBranches(projectCode: string) {
  return useQuery<ProjectBranchView[], ApiError>({
    queryKey: queryKeys.project.branches(projectCode),
    queryFn: () => api.listProjectBranches(projectCode),
    enabled: projectCode !== null && projectCode !== "",
  });
}

/**
 * Create a branch. The list is invalidated on success so the new row
 * shows up in the table without a manual refetch.
 */
export function useCreateProjectBranch(projectCode: string) {
  const qc = useQueryClient();
  return useMutation<ProjectBranchView, ApiError, CreateProjectBranchInput>({
    mutationFn: (input) => api.createProjectBranch(projectCode, input),
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: queryKeys.project.branches(projectCode) });
    },
  });
}

/**
 * Rename a branch. The list is invalidated on success so the new name
 * shows up in the table and in the workspace header chip.
 */
export function useUpdateProjectBranch(projectCode: string) {
  const qc = useQueryClient();
  return useMutation<
    ProjectBranchView,
    ApiError,
    { id: number; body: UpdateProjectBranchInput }
  >({
    mutationFn: ({ id, body }) =>
      api.updateProjectBranch(projectCode, id, body),
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: queryKeys.project.branches(projectCode) });
    },
  });
}

/**
 * The id of the branch this installation is working on for the
 * project, or `null` when there is none.
 *
 * This is desktop-local state, read from the settings store: the server
 * does not model an active branch. `staleTime: Infinity` because the
 * value only changes when this app writes it.
 */
export function useActiveBranchId(projectCode: string) {
  return useQuery<number | null, Error>({
    queryKey: queryKeys.project.activeBranch(projectCode),
    queryFn: () => readActiveBranch(projectCode),
    staleTime: Infinity,
    enabled: projectCode !== null && projectCode !== "",
  });
}

/**
 * Make a branch the active one, or clear the selection with `null`.
 *
 * Writes the settings store and updates the cached value, so the
 * switch and the workspace header chip both react without a refetch.
 */
export function useSetActiveBranchId(projectCode: string) {
  const qc = useQueryClient();
  return useMutation<void, Error, number | null>({
    mutationFn: (branchId) => writeActiveBranch(projectCode, branchId),
    onSuccess: (_void, branchId) => {
      qc.setQueryData(queryKeys.project.activeBranch(projectCode), branchId);
    },
  });
}

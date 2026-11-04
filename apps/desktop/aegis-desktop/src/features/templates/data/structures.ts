import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";

import {
  api,
  type ApiError,
  type CreateStructureInput,
  type StructureNodeInput,
  type StructureNodeView,
  type StructureSummaryView,
  type StructureView,
  type UpdateStructureInput,
} from "../../../shared/api";
import { queryKeys } from "../../../shared/query";

/** The template-level structures, for the sidebar. */
export function useListTemplateStructures() {
  return useQuery<StructureSummaryView[], ApiError>({
    queryKey: queryKeys.structures.list("template"),
    queryFn: () => api.listStructures("template"),
  });
}

/** One structure with its effective tree. */
export function useGetStructure(id: number | null) {
  return useQuery<StructureView, ApiError>({
    queryKey: queryKeys.structures.detail(id ?? 0),
    queryFn: () => api.getStructure(id!),
    enabled: id != null && id > 0,
  });
}

/** Invalidate the structure and the sidebar after a node write. */
function useInvalidateStructures() {
  const qc = useQueryClient();
  return (structureId?: number) => {
    if (structureId != null) {
      qc.invalidateQueries({
        queryKey: queryKeys.structures.detail(structureId),
      });
    }
    qc.invalidateQueries({ queryKey: queryKeys.structures.list("template") });
  };
}

export function useCreateStructure() {
  const invalidate = useInvalidateStructures();
  return useMutation<StructureView, ApiError, CreateStructureInput>({
    mutationFn: (input) => api.createStructure(input),
    onSuccess: () => invalidate(),
  });
}

export function useUpdateStructure() {
  const invalidate = useInvalidateStructures();
  return useMutation<
    StructureView,
    ApiError,
    { id: number; input: UpdateStructureInput }
  >({
    mutationFn: ({ id, input }) => api.updateStructure(id, input),
    onSuccess: (_view, vars) => invalidate(vars.id),
  });
}

export function useDeleteStructure() {
  const qc = useQueryClient();
  const invalidate = useInvalidateStructures();
  return useMutation<void, ApiError, number>({
    mutationFn: (id) => api.deleteStructure(id),
    onSuccess: (_void, id) => {
      // Drop it from the cached list straight away. The invalidation
      // below refetches it, but until that lands the stale entry would
      // still be there — and the page's auto-select could pick the
      // structure that was just deleted.
      qc.setQueryData<StructureSummaryView[]>(
        queryKeys.structures.list("template"),
        (old) => (old ?? []).filter((s) => s.id !== id),
      );
      invalidate(id);
    },
  });
}

export function useAddStructureNode() {
  const invalidate = useInvalidateStructures();
  return useMutation<
    StructureNodeView,
    ApiError,
    { structureId: number; input: StructureNodeInput }
  >({
    mutationFn: ({ structureId, input }) =>
      api.addStructureNode(structureId, input),
    onSuccess: (_node, vars) => invalidate(vars.structureId),
  });
}

export function useUpdateStructureNode() {
  const invalidate = useInvalidateStructures();
  return useMutation<
    StructureNodeView,
    ApiError,
    { structureId: number; nodeId: string; input: StructureNodeInput }
  >({
    mutationFn: ({ structureId, nodeId, input }) =>
      api.updateStructureNode(structureId, nodeId, input),
    onSuccess: (_node, vars) => invalidate(vars.structureId),
  });
}

export function useDeleteStructureNode() {
  const invalidate = useInvalidateStructures();
  return useMutation<void, ApiError, { structureId: number; nodeId: string }>({
    mutationFn: ({ structureId, nodeId }) =>
      api.deleteStructureNode(structureId, nodeId),
    onSuccess: (_void, vars) => invalidate(vars.structureId),
  });
}

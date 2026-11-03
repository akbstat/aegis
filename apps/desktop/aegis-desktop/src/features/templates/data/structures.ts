import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";

import {
  api,
  type ApiError,
  type StructureNodeInput,
  type StructureNodeView,
  type StructureSummaryView,
  type StructureView,
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
  return (structureId: number) => {
    qc.invalidateQueries({ queryKey: queryKeys.structures.detail(structureId) });
    qc.invalidateQueries({ queryKey: queryKeys.structures.list("template") });
  };
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

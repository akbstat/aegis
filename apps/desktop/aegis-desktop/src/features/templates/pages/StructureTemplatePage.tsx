import { useEffect, useState } from "react";
import { useNavigate } from "@tanstack/react-router";
import { Box, Typography } from "@aegis/ui/mui";
import { useI18n } from "@aegis/ui/i18n";

import { useCurrentUser } from "../../auth";
import type { StructureNodeInput, StructureNodeView } from "../../../shared/api";
import {
  useAddStructureNode,
  useDeleteStructureNode,
  useGetStructure,
  useListTemplateStructures,
  useUpdateStructureNode,
} from "../data/structures";
import { DeleteNodeDialog } from "../components/DeleteNodeDialog";
import { NodeDrawer } from "../components/NodeDrawer";
import { StructureSidebar } from "../components/StructureSidebar";
import { StructureTree } from "../components/StructureTree";

export interface StructureTemplatePageProps {
  /** `null` on the index route — the first template is selected then. */
  structureId: number | null;
}

export function StructureTemplatePage({
  structureId,
}: StructureTemplatePageProps) {
  const { t } = useI18n();
  const navigate = useNavigate();
  const currentUser = useCurrentUser();

  const structures = useListTemplateStructures();
  const structure = useGetStructure(structureId);

  const addNode = useAddStructureNode();
  const updateNode = useUpdateStructureNode();
  const deleteNode = useDeleteStructureNode();

  const [drawerOpen, setDrawerOpen] = useState(false);
  const [drawerMode, setDrawerMode] = useState<"create" | "edit">("create");
  const [drawerNode, setDrawerNode] = useState<StructureNodeView | null>(null);
  const [drawerParent, setDrawerParent] = useState<StructureNodeView | null>(
    null,
  );
  const [deleteTarget, setDeleteTarget] = useState<StructureNodeView | null>(
    null,
  );

  const canMutate =
    currentUser.data?.role === "root" || currentUser.data?.role === "admin";

  // With no structure selected, settle on the first template so the URL
  // becomes a shareable link.
  useEffect(() => {
    if (structureId != null) return;
    const first = structures.data?.[0];
    if (!first) return;
    navigate({
      to: "/templates/structure/$structureId",
      params: { structureId: String(first.id) },
      replace: true,
    });
  }, [structureId, structures.data, navigate]);

  function openAdd(parent: StructureNodeView | null) {
    setDrawerMode("create");
    setDrawerNode(null);
    setDrawerParent(parent);
    setDrawerOpen(true);
  }

  function openEdit(node: StructureNodeView) {
    setDrawerMode("edit");
    setDrawerNode(node);
    setDrawerParent(null);
    setDrawerOpen(true);
  }

  function handleSubmit(input: StructureNodeInput) {
    if (structureId == null) return;
    // The drawer stays open until the write succeeds, so a failure shows
    // the server's message in the drawer instead of vanishing.
    const onSuccess = () => setDrawerOpen(false);
    if (drawerMode === "create") {
      addNode.mutate({ structureId, input }, { onSuccess });
    } else if (drawerNode) {
      updateNode.mutate(
        { structureId, nodeId: drawerNode.nodeId, input },
        { onSuccess },
      );
    }
  }

  function handleDelete(node: StructureNodeView) {
    if (structureId == null) return;
    deleteNode.mutate(
      { structureId, nodeId: node.nodeId },
      { onSuccess: () => setDeleteTarget(null) },
    );
  }

  return (
    <Box sx={{ display: "flex", minHeight: "100vh" }}>
      <Box
        sx={{
          flexGrow: 1,
          minWidth: 0,
          display: "flex",
          flexDirection: "column",
        }}
      >
        <Typography variant="h5" sx={{ p: 3, pb: 0 }}>
          {structure.data?.name ?? t("templates.structure.heading")}
        </Typography>
        <StructureTree
          nodes={structure.data?.nodes ?? []}
          canMutate={canMutate}
          onAdd={openAdd}
          onEdit={openEdit}
          onDelete={setDeleteTarget}
        />
      </Box>

      <StructureSidebar
        structures={structures.data ?? []}
        selectedId={structureId}
        onSelect={(id) =>
          navigate({
            to: "/templates/structure/$structureId",
            params: { structureId: String(id) },
          })
        }
        loading={structures.isLoading}
      />

      <NodeDrawer
        open={drawerOpen}
        mode={drawerMode}
        node={drawerNode}
        parent={drawerParent}
        onClose={() => setDrawerOpen(false)}
        onSubmit={handleSubmit}
        pending={addNode.isPending || updateNode.isPending}
        error={addNode.error ?? updateNode.error}
      />

      <DeleteNodeDialog
        open={deleteTarget != null}
        node={deleteTarget}
        onClose={() => setDeleteTarget(null)}
        onConfirm={handleDelete}
        pending={deleteNode.isPending}
        error={deleteNode.error}
      />
    </Box>
  );
}

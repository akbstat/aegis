import { useEffect, useState } from "react";
import { useNavigate } from "@tanstack/react-router";
import { Box, Typography } from "@aegis/ui/mui";
import { useI18n } from "@aegis/ui/i18n";

import { useCurrentUser } from "../../auth";
import type {
  StructureNodeInput,
  StructureNodeView,
  StructureSummaryView,
} from "../../../shared/api";
import {
  useAddStructureNode,
  useCreateStructure,
  useDeleteStructure,
  useDeleteStructureNode,
  useGetStructure,
  useListTemplateStructures,
  useUpdateStructure,
  useUpdateStructureNode,
} from "../data/structures";
import { DeleteNodeDialog } from "../components/DeleteNodeDialog";
import { DeleteStructureDialog } from "../components/DeleteStructureDialog";
import { NodeDrawer } from "../components/NodeDrawer";
import { StructureDrawer, type StructureFormValue } from "../components/StructureDrawer";
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

  const createStructure = useCreateStructure();
  const updateStructure = useUpdateStructure();
  const deleteStructure = useDeleteStructure();

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

  const [structureDrawerOpen, setStructureDrawerOpen] = useState(false);
  const [structureDrawerMode, setStructureDrawerMode] = useState<
    "create" | "update"
  >("create");
  const [structureDrawerTarget, setStructureDrawerTarget] =
    useState<StructureSummaryView | null>(null);
  const [structureDeleteTarget, setStructureDeleteTarget] =
    useState<StructureSummaryView | null>(null);

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
    // A mutation error belongs to the write that failed. Clear it so a
    // freshly opened drawer never shows the previous failure.
    addNode.reset();
    updateNode.reset();
    setDrawerMode("create");
    setDrawerNode(null);
    setDrawerParent(parent);
    setDrawerOpen(true);
  }

  function openEdit(node: StructureNodeView) {
    addNode.reset();
    updateNode.reset();
    setDrawerMode("edit");
    setDrawerNode(node);
    setDrawerParent(null);
    setDrawerOpen(true);
  }

  function openCreateStructure() {
    createStructure.reset();
    updateStructure.reset();
    setStructureDrawerMode("create");
    setStructureDrawerTarget(null);
    setStructureDrawerOpen(true);
  }

  function openUpdateStructure(structure: StructureSummaryView) {
    createStructure.reset();
    updateStructure.reset();
    setStructureDrawerMode("update");
    setStructureDrawerTarget(structure);
    setStructureDrawerOpen(true);
  }

  function openDeleteStructure(structure: StructureSummaryView) {
    deleteStructure.reset();
    setStructureDeleteTarget(structure);
  }

  function handleStructureSubmit(value: StructureFormValue) {
    if (structureDrawerMode === "create") {
      createStructure.mutate(
        {
          name: value.name,
          // The drawer keeps the submit button disabled until a root
          // path is picked, so this is never empty.
          rootpath: value.rootpath ?? "",
          level: "template",
          projectCode: null,
          templateId: null,
        },
        { onSuccess: () => setStructureDrawerOpen(false) },
      );
      return;
    }
    if (!structureDrawerTarget) return;
    updateStructure.mutate(
      {
        id: structureDrawerTarget.id,
        // A `null` root path leaves the stored one alone.
        input: { name: value.name, rootpath: value.rootpath },
      },
      { onSuccess: () => setStructureDrawerOpen(false) },
    );
  }

  function handleStructureDelete(structure: StructureSummaryView) {
    deleteStructure.mutate(structure.id, {
      onSuccess: () => {
        setStructureDeleteTarget(null);
        // The deleted structure may be the one on screen. Go back to
        // the index route, where the auto-select effect picks another
        // one — the deleted one is already out of the cached list.
        if (structure.id !== structureId) return;
        navigate({ to: "/templates/structure" });
      },
    });
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

  function openDelete(node: StructureNodeView) {
    deleteNode.reset();
    setDeleteTarget(node);
  }

  function handleDelete(node: StructureNodeView) {
    if (structureId == null) return;
    deleteNode.mutate(
      { structureId, nodeId: node.nodeId },
      { onSuccess: () => setDeleteTarget(null) },
    );
  }

  return (
    // A bounded height: the page is exactly one viewport tall, so the
    // tree pane scrolls inside it and the heading, the toggle and the
    // sidebar stay put.
    <Box sx={{ display: "flex", height: "100vh", overflow: "hidden" }}>
      <Box
        sx={{
          flexGrow: 1,
          minWidth: 0,
          minHeight: 0,
          display: "flex",
          flexDirection: "column",
        }}
      >
        <Typography variant="h5" sx={{ p: 3, pb: 0, flexShrink: 0 }}>
          {structure.data?.name ?? t("templates.structure.heading")}
        </Typography>
        <StructureTree
          nodes={structure.data?.nodes ?? []}
          canMutate={canMutate}
          canAddRoot={structureId != null}
          loading={structure.isLoading}
          onAdd={openAdd}
          onEdit={openEdit}
          onDelete={openDelete}
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
        onCreate={openCreateStructure}
        onUpdate={openUpdateStructure}
        onDelete={openDeleteStructure}
        canMutate={canMutate}
        loading={structures.isLoading}
      />

      <StructureDrawer
        open={structureDrawerOpen}
        mode={structureDrawerMode}
        structure={structureDrawerTarget}
        onClose={() => setStructureDrawerOpen(false)}
        onSubmit={handleStructureSubmit}
        pending={createStructure.isPending || updateStructure.isPending}
        error={createStructure.error ?? updateStructure.error}
      />

      <DeleteStructureDialog
        open={structureDeleteTarget != null}
        structure={structureDeleteTarget}
        onClose={() => setStructureDeleteTarget(null)}
        onConfirm={handleStructureDelete}
        pending={deleteStructure.isPending}
        error={deleteStructure.error}
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

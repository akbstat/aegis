import { createFileRoute } from "@tanstack/react-router";
import { StructureTemplatePage } from "../../../../../features/templates";

export const Route = createFileRoute("/_authed/_layout/templates/structure/")({
  component: () => <StructureTemplatePage structureId={null} />,
});

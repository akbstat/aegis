import { createFileRoute } from "@tanstack/react-router";
import { TemplatesPage } from "../../../../features/templates";

export const Route = createFileRoute("/_authed/_layout/templates/")({
  component: TemplatesPage,
});

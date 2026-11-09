import { createRoute } from "@tanstack/react-router";
import { ProjectsPage } from "@/components/ProjectsPage";
import { rootRoute } from "./__root";

export const workspaceRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: "/",
  component: ProjectsPage,
});

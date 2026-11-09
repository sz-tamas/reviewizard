import { createRoute } from "@tanstack/react-router";
import { rootRoute } from "./__root";
import { ProjectsPage } from "../components/ProjectsPage";

export const projectsRoute = createRoute({ getParentRoute: () => rootRoute, path: "/projects", component: ProjectsPage });

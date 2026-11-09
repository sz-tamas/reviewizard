import { createRouter, type RouterHistory } from "@tanstack/react-router";
import type { QueryClient } from "@tanstack/react-query";
import { rootRoute } from "@/routes/__root";
import { workspaceRoute } from "@/routes/index";
import { projectsRoute } from "@/routes/projects";
import { reviewsRoute } from "@/routes/reviews";

const routeTree = rootRoute.addChildren([workspaceRoute, projectsRoute, reviewsRoute]);

export function createAppRouter(queryClient: QueryClient, history?: RouterHistory) {
  return createRouter({
    routeTree,
    context: { queryClient },
    history,
    defaultPreload: "intent",
    // Query owns data freshness when route loaders use ensureQueryData.
    defaultPreloadStaleTime: 0,
  });
}

declare module "@tanstack/react-router" {
  interface Register {
    router: ReturnType<typeof createAppRouter>;
  }
}

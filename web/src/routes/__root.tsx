import { createRootRouteWithContext, Link, Outlet } from "@tanstack/react-router";
import type { QueryClient } from "@tanstack/react-query";
import { Button } from "@/components/ui/button";

export const rootRoute = createRootRouteWithContext<{ queryClient: QueryClient }>()({
  component: Outlet,
  notFoundComponent: NotFound,
});

function NotFound() {
  return (
    <main className="flex min-h-screen flex-col items-center justify-center gap-4 bg-background p-8 text-foreground">
      <h1 className="text-xl font-semibold">Page not found</h1>
      <p className="text-muted-foreground">Return to the review workspace to continue.</p>
      <Button asChild>
        <Link to="/reviews">Open review workspace</Link>
      </Button>
    </main>
  );
}

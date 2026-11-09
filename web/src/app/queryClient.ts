import { QueryClient } from "@tanstack/react-query";

// Create one client per app mount (and per test), never a process-global cache.
export function createQueryClient() {
  return new QueryClient({
    defaultOptions: {
      queries: {
        staleTime: 30_000,
        retry: false,
        refetchOnWindowFocus: false,
        refetchOnReconnect: false,
      },
      // Retrying a write can duplicate a comment or review operation.
      mutations: { retry: false },
    },
  });
}

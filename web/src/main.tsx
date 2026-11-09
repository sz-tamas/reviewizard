import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import { QueryClientProvider } from "@tanstack/react-query";
import { RouterProvider } from "@tanstack/react-router";
import { createQueryClient } from "./app/queryClient";
import { createAppRouter } from "./app/router";
import { ErrorBoundary } from "./components/ErrorBoundary";
import "./styles.css";
// Revert the teal/compact appearance by removing this one import.
import "./compact-cyan.css";

const queryClient = createQueryClient();
const router = createAppRouter(queryClient);

createRoot(document.getElementById("root")!).render(
  <StrictMode>
    <ErrorBoundary>
      <QueryClientProvider client={queryClient}>
        <RouterProvider router={router} />
      </QueryClientProvider>
    </ErrorBoundary>
  </StrictMode>
);

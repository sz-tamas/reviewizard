import { afterEach, beforeEach, expect, test, vi } from "vitest";
import { fireEvent, render, screen, within } from "@testing-library/react";
import { QueryClientProvider } from "@tanstack/react-query";
import { createMemoryHistory, RouterProvider } from "@tanstack/react-router";
import { createQueryClient } from "./queryClient";
import { createAppRouter } from "./router";

vi.mock("../api/api", async (original) => {
  const mod = await original<typeof import("../api/api")>();
  return { ...mod, api: { ...mod.api, repos: vi.fn(async () => ({ repos: [] })), projects: vi.fn(async () => []), getReview: vi.fn(), branches: vi.fn(), commits: vi.fn(), reviewDiff: vi.fn(), createReview: vi.fn(), setReviewStatus: vi.fn(async () => undefined) } };
});

import { api } from "../api/api";

beforeEach(() => {
  vi.spyOn(window, "scrollTo").mockImplementation(() => {});
});

afterEach(() => {
  vi.restoreAllMocks();
  vi.clearAllMocks();
  localStorage.clear();
});

function renderWorkspace(path: string) {
  const client = createQueryClient();
  const router = createAppRouter(client, createMemoryHistory({ initialEntries: [path] }));
  render(
    <QueryClientProvider client={client}>
      <RouterProvider router={router} />
    </QueryClientProvider>
  );
  return router;
}

test("the index route opens Projects without starting a review", async () => {
  renderWorkspace("/");
  expect(await screen.findByRole("heading", { name: "Add your first project" })).toBeTruthy();
  expect(api.projects).toHaveBeenCalledTimes(1);
  expect(api.repos).not.toHaveBeenCalled();
});

test("the reviews route asks for a saved review instead of starting another comparison", async () => {
  renderWorkspace("/reviews");
  expect(await screen.findByRole("heading", { name: "Choose a saved review" })).toBeTruthy();
  expect(api.repos).not.toHaveBeenCalled();
});

test("an unknown URL offers a shadcn link back to the working review route", async () => {
  const router = renderWorkspace("/missing");
  expect(await screen.findByRole("heading", { name: "Page not found" })).toBeTruthy();
  expect(api.repos).not.toHaveBeenCalled();
  const link = screen.getByRole("link", { name: "Open review workspace" });
  expect(link.getAttribute("href")).toBe("/reviews");
  expect(link.getAttribute("data-slot")).toBe("button");
  fireEvent.click(link);
  expect(await screen.findByRole("heading", { name: "Choose a saved review" })).toBeTruthy();
  expect(router.state.location.pathname).toBe("/reviews");
});


test("a saved review route keeps the original comments sidebar and loads the saved diff", async () => {
  vi.mocked(api.repos).mockResolvedValue({ repos: [{ name: "@project/7", displayName: "frontend", path: "/work/frontend", lastActivity: "" }] });
  vi.mocked(api.getReview).mockResolvedValue({ id: 70, repoPath: "/work/frontend", headRef: "feature", baseRef: "main", headSha: "sha", scopeFrom: "all", scopeSide: "index", scopeConfigured: true, status: "draft", comments: [], reviewedFiles: [], summary: "", createdAt: "", updatedAt: "" });
  vi.mocked(api.branches).mockResolvedValue({ branches: [{ name: "feature", isCurrent: true, isMain: false, isRemote: false, lastCommit: "" }, { name: "main", isCurrent: false, isMain: true, isRemote: false, lastCommit: "" }] });
  vi.mocked(api.commits).mockResolvedValue({ commits: [] });
  vi.mocked(api.reviewDiff).mockResolvedValue({ base: "parent", head: "feature", files: [] });
  renderWorkspace("/reviews?review=70");
  expect(await screen.findByRole("heading", { name: /Comments/ })).toBeTruthy();
  expect(screen.getByText("+ Add a review summary")).toBeTruthy();
  expect(screen.getByText(/Review: Staged/)).toBeTruthy();
  expect(screen.queryByRole("heading", { name: "AI Review Handoff" })).toBeNull();
  expect(screen.queryByLabelText("head branch")).toBeNull();
  expect(api.createReview).not.toHaveBeenCalled();
  expect(api.reviewDiff).toHaveBeenCalledWith(70);
  fireEvent.click(screen.getByRole("button", { name: "AI Review Handoff" }));
  const handoff = screen.getByRole("dialog", { name: "AI Review Handoff" });
  expect(within(handoff).getAllByRole("checkbox")).toHaveLength(6);
  expect(within(handoff).getByRole("button", { name: "Generate Handoff Prompt" })).toBeTruthy();
  expect(within(handoff).queryByRole("button", { name: "Address the review" })).toBeNull();
  fireEvent.click(within(handoff).getByRole("button", { name: "Close", exact: true }));
  fireEvent.click(screen.getByRole("button", { name: "Complete Review", exact: true }));
  const completion = screen.getByRole("dialog", { name: "Complete Review" });
  expect(api.setReviewStatus).not.toHaveBeenCalled();
  expect(within(completion).getByRole("region", { name: "Close manually" })).toBeTruthy();
  expect(within(completion).getByRole("textbox")).toBeTruthy();
  expect(within(completion).getByText("0 / 0 (0%)")).toBeTruthy();
  fireEvent.click(within(completion).getByRole("button", { name: "Complete review", exact: true }));
  expect(await screen.findByRole("button", { name: "Reopen Review" })).toBeTruthy();
  expect(api.setReviewStatus).toHaveBeenCalledWith(70, "completed");

});

test("opening a saved codebase review generates an entire-codebase handoff", async () => {
  vi.mocked(api.repos).mockResolvedValue({ repos: [{ name: "@project/7", displayName: "frontend", path: "/work/frontend", lastActivity: "" }] });
  vi.mocked(api.getReview).mockResolvedValue({ id: 71, repoPath: "/work/frontend", headRef: "HEAD", baseRef: "", headSha: "sha", scopeMode: "codebase", scopeFrom: "all", scopeSide: "worktree", scopeConfigured: true, status: "draft", comments: [], reviewedFiles: [], summary: "", createdAt: "", updatedAt: "" });
  vi.mocked(api.branches).mockResolvedValue({ branches: [] });
  vi.mocked(api.commits).mockResolvedValue({ commits: [] });
  vi.mocked(api.reviewDiff).mockResolvedValue({ base: "empty-tree", head: "HEAD", files: [] });
  renderWorkspace("/reviews?review=71");
  expect(await screen.findByText(/Scope: Entire codebase/)).toBeTruthy();
  expect(api.reviewDiff).toHaveBeenCalledWith(71);
  expect(api.createReview).not.toHaveBeenCalled();
  fireEvent.click(screen.getByRole("button", { name: "AI Review Handoff" }));
  const handoff = screen.getByRole("dialog", { name: "AI Review Handoff" });
  fireEvent.click(within(handoff).getByRole("button", { name: "Generate Handoff Prompt" }));
  const output = (within(handoff).getByRole("textbox", { name: "Generated handoff prompt" }) as HTMLTextAreaElement).value;
  expect(output).toContain("Review the entire codebase");
  expect(output).toContain("including unchanged files");
  expect(output).toContain("/api/reviews/71/diff");
  expect(output).toContain('"side":"worktree"');
});

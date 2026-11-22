import { beforeEach, expect, test, vi } from "vitest";
import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { QueryClientProvider } from "@tanstack/react-query";
import { createQueryClient } from "../app/queryClient";
import { api } from "../api/api";
import { ReviewSetupModal } from "./ReviewSetupModal";
import type { Project, Review } from "../api/types";

vi.mock("../api/api", async (original) => {
  const mod = await original<typeof import("../api/api")>();
  return { ...mod, api: { ...mod.api, branches: vi.fn(), commits: vi.fn(), saveProjectReview: vi.fn() } };
});
const project: Project = { id: 7, name: "app", path: "/work/app", reviewCount: 0, openFindings: 0, resolvedFindings: 0, lastActivity: "" };
const other = { ...project, id: 8, name: "service", path: "/work/service" };
const saved = { id: 70, repoPath: project.path, headRef: "feature", baseRef: "main", scopeSide: "index", scopeFrom: "abc123", scopeConfigured: true } as Review;
const onSaved = vi.fn();
const onClose = vi.fn();
const branch = (name: string, current = false, main = false, remote = false) => ({ name, isCurrent: current, isMain: main, isRemote: remote, lastCommit: "" });
beforeEach(() => {
  vi.clearAllMocks();
  vi.mocked(api.branches).mockResolvedValue({ branches: [branch("feature", true), branch("main", false, true), branch("origin/main", false, false, true)] });
  vi.mocked(api.commits).mockResolvedValue({ commits: [{ sha: "abc123", shortSha: "abc123", subject: "Selected change", relDate: "" }] });
  vi.mocked(api.saveProjectReview).mockResolvedValue(saved);
});
function mount() {
  render(<QueryClientProvider client={createQueryClient()}><ReviewSetupModal projects={[project, other]} initialProject={project} onClose={onClose} onSaved={onSaved} /></QueryClientProvider>);
}

async function openDropdown(label: string) {
  const trigger = screen.getByRole("button", { name: label });
  await waitFor(() => expect((trigger as HTMLButtonElement).disabled).toBe(false));
  fireEvent.keyDown(trigger, { key: "ArrowDown" });
  return screen.findByRole("menu");
}

async function choose(label: string, option: string) {
  await openDropdown(label);
  fireEvent.click(await screen.findByRole("menuitemradio", { name: option }));
}

test("Escape closes an open picker before closing review setup", async () => {
  mount();
  const menu = await openDropdown("Review repository");
  expect(screen.getByRole("menuitemradio", { name: "app — /work/app", checked: true })).toBeTruthy();
  fireEvent.keyDown(menu, { key: "Escape" });
  expect(screen.queryByRole("menu")).toBeNull();
  expect(onClose).not.toHaveBeenCalled();
  fireEvent.keyDown(screen.getByRole("button", { name: "Review repository" }), { key: "Escape" });
  expect(onClose).toHaveBeenCalledOnce();
});

test("saving carries repository, head, explicit base, inclusive commit and source side", async () => {
  mount();
  await waitFor(() => expect(screen.getByRole("button", { name: "Review head branch" }).textContent).toBe("feature"));
  await choose("Review base branch", "origin/main (remote, last fetched)");
  await choose("Review starting commit", "abc123 — Selected change");
  fireEvent.click(screen.getByRole("button", { name: "Staged" }));
  fireEvent.click(screen.getByRole("button", { name: "Save review" }));
  await waitFor(() => expect(api.saveProjectReview).toHaveBeenCalledWith(7, { mode: "changes", head: "feature", base: "origin/main", from: "abc123", side: "index" }));
  await waitFor(() => expect(onSaved).toHaveBeenCalledWith(saved));
});

test("uncommitted sources are unavailable off the checked-out branch", async () => {
  mount();
  await waitFor(() => expect(screen.getByRole("button", { name: "Review head branch" }).textContent).toBe("feature"));
  await choose("Review head branch", "main");
  expect((screen.getByRole("button", { name: "Staged" }) as HTMLButtonElement).disabled).toBe(true);
  expect((screen.getByRole("button", { name: "Working tree" }) as HTMLButtonElement).disabled).toBe(true);
  expect(screen.getByRole("button", { name: "Committed" }).getAttribute("aria-pressed")).toBe("true");
});

test("changing repositories drops the other repository's branch and starting commit", async () => {
  vi.mocked(api.branches).mockImplementation(async (repo) => ({ branches: repo === "@project/7" ? [branch("feature", true), branch("main", false, true)] : [branch("develop", true), branch("main", false, true)] }));
  mount();
  await choose("Review starting commit", "abc123 — Selected change");
  fireEvent.click(screen.getByRole("button", { name: "Staged" }));
  await choose("Review repository", "service — /work/service");
  await waitFor(() => expect(screen.getByRole("button", { name: "Review head branch" }).textContent).toBe("develop"));
  fireEvent.click(screen.getByRole("button", { name: "Save review" }));
  await waitFor(() => expect(api.saveProjectReview).toHaveBeenCalledWith(8, { mode: "changes", head: "develop", base: "", from: "all", side: "head" }));
});

test("failed scope validation keeps the setup open without reporting a saved review", async () => {
  vi.mocked(api.saveProjectReview).mockRejectedValue(new Error("Unknown ref: deleted-base"));
  mount();
  await waitFor(() => expect(screen.getByRole("button", { name: "Review head branch" }).textContent).toBe("feature"));
  fireEvent.click(screen.getByRole("button", { name: "Save review" }));
  expect((await screen.findByRole("alert")).textContent).toBe("Unknown ref: deleted-base");
  expect(onSaved).not.toHaveBeenCalled();
});

test("codebase scope hides branch and source controls and saves the current codebase", async () => {
  mount();
  await choose("Review base branch", "origin/main (remote, last fetched)");
  await choose("Review starting commit", "abc123 — Selected change");
  fireEvent.click(screen.getByRole("button", { name: "Codebase" }));
  expect(screen.queryByLabelText("Review base branch")).toBeNull();
  expect(screen.queryByLabelText("Review starting commit")).toBeNull();
  expect(screen.queryByLabelText("Review head branch")).toBeNull();
  expect(screen.queryByRole("group", { name: "Review source side" })).toBeNull();
  fireEvent.click(screen.getByRole("button", { name: "Save review" }));
  await waitFor(() => expect(api.saveProjectReview).toHaveBeenCalledWith(7, { mode: "codebase", head: "HEAD", base: "", from: "all", side: "worktree" }));
});

test("codebase uses the checked-out branch even after choosing a different changes branch", async () => {
  mount();
  await waitFor(() => expect(screen.getByRole("button", { name: "Review head branch" }).textContent).toBe("feature"));
  await choose("Review head branch", "main");
  fireEvent.click(screen.getByRole("button", { name: "Codebase" }));
  fireEvent.click(screen.getByRole("button", { name: "Save review" }));
  await waitFor(() => expect(api.saveProjectReview).toHaveBeenCalledWith(7, { mode: "codebase", head: "HEAD", base: "", from: "all", side: "worktree" }));
});

for (const failed of [false, true]) {
  test(`codebase saves without branch data (branch request fails: ${failed})`, async () => {
    if (failed) vi.mocked(api.branches).mockRejectedValue(new Error("Branch lookup failed"));
    else vi.mocked(api.branches).mockResolvedValue({ branches: [] });
    mount();
    fireEvent.click(screen.getByRole("button", { name: "Codebase" }));
    const save = screen.getByRole("button", { name: "Save review" }) as HTMLButtonElement;
    expect(save.disabled).toBe(false);
    fireEvent.click(save);
    await waitFor(() => expect(api.saveProjectReview).toHaveBeenCalledWith(7, { mode: "codebase", head: "HEAD", base: "", from: "all", side: "worktree" }));
    await waitFor(() => expect(onSaved).toHaveBeenCalledWith(saved));
  });
}

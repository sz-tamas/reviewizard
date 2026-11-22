import { beforeEach, expect, test, vi } from "vitest";
import { fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { QueryClientProvider } from "@tanstack/react-query";
import { createQueryClient } from "../app/queryClient";
import { api } from "../api/api";
import { ProjectsPage } from "./ProjectsPage";

vi.mock("../api/api", async (original) => {
  const mod = await original<typeof import("../api/api")>();
  return { ...mod, api: { ...mod.api, projects: vi.fn(), branches: vi.fn(), addProject: vi.fn(), removeProject: vi.fn(), projectFolders: vi.fn(), projectReviews: vi.fn(), getReview: vi.fn(), reviewDiff: vi.fn(), removeReview: vi.fn(), commits: vi.fn(), saveProjectReview: vi.fn() } };
});
const project = { id: 7, name: "frontend", path: "/home/me/frontend", reviewCount: 12, openFindings: 3, resolvedFindings: 9, lastActivity: "2026-11-08T12:00:00Z" };
beforeEach(() => {
  vi.clearAllMocks();
  vi.mocked(api.projects).mockResolvedValue([project]);
  vi.mocked(api.branches).mockResolvedValue({ branches: [
    { name: "main", isCurrent: true, isMain: true, isRemote: false, lastCommit: "" },
    { name: "feature/auth", isCurrent: false, isMain: false, isRemote: false, lastCommit: "" },
  ] });
  vi.mocked(api.projectReviews).mockResolvedValue([{ id: 17, repoPath: project.path, headRef: "feature/auth", baseRef: "main", scopeFrom: "all", scopeSide: "index", status: "draft", headSha: "sha", createdAt: "2026-11-01T12:00:00Z", updatedAt: "2026-11-08T12:00:00Z", summary: "", comments: [], reviewedFiles: [] }]);
  vi.mocked(api.getReview).mockImplementation(async (id) => (await api.projectReviews(7)).find((review) => review.id === id)!);
  vi.mocked(api.reviewDiff).mockResolvedValue({ base: "base", head: "head", files: [] });
  vi.mocked(api.commits).mockResolvedValue({ commits: [] });
  vi.mocked(api.projectFolders).mockResolvedValue({ path: "/home/me", roots: ["/home/me"], parent: null, isRepository: false, folders: [{ name: "frontend", path: project.path, isRepository: true }] });
});
function mount() {
  const client = createQueryClient();
  render(<QueryClientProvider client={client}><ProjectsPage /></QueryClientProvider>);
  return client;
}

test("project metadata shows root findings and distinguishes missing files", async () => {
  vi.mocked(api.projects).mockResolvedValue([{
    ...project,
    metadata: {
      readme: "README.md",
      license: null,
      branch: "feature/auth",
      languages: [{ name: "Rust", version: "1.94.1" }, { name: "TypeScript", version: null }],
    },
  }]);
  mount();
  const row = await screen.findByRole("list", { name: "Repository metadata" });
  for (const label of ["README.md", "No license found", "feature/auth", "Rust 1.94.1", "TypeScript"]) {
    expect(within(row).getByText(label)).toBeTruthy();
  }
  expect(row.querySelector('[data-language="Rust"] .tabler-icon-brand-rust')).toBeTruthy();
  expect(row.querySelector('[data-language="TypeScript"] .tabler-icon-brand-typescript')).toBeTruthy();
});

test("cards show persisted counts and carry the selected project and branch to Reviews", async () => {
  mount();
  await screen.findByText(project.path);
  expect(screen.getByText("12 reviews")).toBeTruthy();
  expect((await screen.findByRole("link", { name: "Open Review", exact: true })).getAttribute("href")).toBe("/reviews?review=17");
  const link = screen.getByRole("link", { name: "Open Review", exact: true });
  expect(link.className).toBe("project-review-link");
  const card = screen.getByRole("region", { name: "Review 17" });
  expect(within(card).getByText("Created").querySelector("time")?.dateTime).toBe("2026-11-01T12:00:00Z");
  expect(within(card).getByText("Last edited").querySelector("time")?.dateTime).toBe("2026-11-08T12:00:00Z");
  expect(screen.queryByText("Open Review")).toBeNull();
  expect(link.contains(screen.getByRole("button", { name: "Remove review 17" }))).toBe(false);
  expect(within(link).getByRole("heading", { name: /frontend\s*\/\s*feature\/auth/ })).toBeTruthy();
  const details = await within(link).findByText(/Review: Staged.*Base: main.*Scope: Changed files \(0\)/);
  expect(details.textContent).not.toContain("In progress");
  expect(await within(card).findByText("Files reviewed")).toBeTruthy();
  expect(within(card).getByText("0 / 0 (0%)")).toBeTruthy();
  const header = screen.getByRole("heading", { name: "Projects", exact: true }).closest("section")!;
  expect(within(header).getByRole("button", { name: "Add Project" })).toBeTruthy();
  expect(screen.queryByLabelText("Search projects")).toBeNull();
});

test("the folder picker registers the selected absolute path and surfaces failed validation", async () => {
  vi.mocked(api.addProject).mockRejectedValueOnce(new Error("Choose a Git repository root containing .git"));
  vi.mocked(api.addProject).mockResolvedValueOnce(project);
  const client = mount();
  await screen.findByText(project.path);
  fireEvent.click(screen.getByRole("button", { name: "Add Project", exact: true }));
  const dialog = screen.getByRole("dialog", { name: "Add Project" });
  await within(dialog).findByRole("button", { name: /frontend.*Git/ });
  fireEvent.change(within(dialog).getByLabelText("Repository folder"), { target: { value: "/home/me/not-git" } });
  fireEvent.click(within(dialog).getByRole("button", { name: "Add Project", exact: true }));
  expect(await within(dialog).findByRole("alert")).toHaveProperty("textContent", "Choose a Git repository root containing .git");
  expect(api.addProject).toHaveBeenCalledWith("/home/me/not-git");
  fireEvent.change(within(dialog).getByLabelText("Repository folder"), { target: { value: project.path } });
  fireEvent.click(within(dialog).getByRole("button", { name: "Add Project", exact: true }));
  await waitFor(() => expect(screen.queryByRole("dialog")).toBeNull());
  expect(api.saveProjectReview).not.toHaveBeenCalled();
  expect(api.branches).not.toHaveBeenCalled();
  const grid = screen.getByLabelText("Saved reviews for frontend");
  fireEvent.click(within(grid).getByRole("button", { name: "Add review", exact: true }));
  expect(await screen.findByRole("dialog", { name: "Set up review" })).toBeTruthy();
  expect(api.addProject).toHaveBeenLastCalledWith(project.path);
  client.clear();
});

test("removing a project requires a concrete confirmation and does not delete a review", async () => {
  vi.mocked(api.removeProject).mockResolvedValue(undefined);
  mount();
  await screen.findByText(project.path);
  const trigger = screen.getByRole("button", { name: "Remove project frontend" });
  fireEvent.click(trigger);
  expect(api.removeProject).not.toHaveBeenCalled();
  const dialog = screen.getByRole("dialog", { name: "Remove project?" });
  expect(dialog.classList.contains("modal-sm")).toBe(true);
  expect(within(dialog).queryByRole("button", { name: "Close" })).toBeNull();
  expect(within(dialog).getByText(/all existing reviews and findings will be kept/)).toBeTruthy();
  fireEvent.click(within(dialog).getByRole("button", { name: "Cancel" }));
  expect(screen.queryByRole("dialog")).toBeNull();
  expect(api.removeProject).not.toHaveBeenCalled();
  fireEvent.click(trigger);
  fireEvent.click(within(screen.getByRole("dialog", { name: "Remove project?" })).getByRole("button", { name: "Remove" }));
  await waitFor(() => expect(api.removeProject).toHaveBeenCalledWith(7));
});

test("browsing into a repository selects its absolute folder for registration", async () => {
  vi.mocked(api.projectFolders).mockImplementation(async (path) => path === project.path
    ? { path: project.path, roots: ["/home/me"], parent: "/home/me", isRepository: true, folders: [] }
    : { path: "/home/me", roots: ["/home/me"], parent: null, isRepository: false, folders: [{ name: "frontend", path: project.path, isRepository: true }] });
  vi.mocked(api.addProject).mockResolvedValue(project);
  mount();
  await screen.findByText(project.path);
  fireEvent.click(screen.getByRole("button", { name: "Add Project", exact: true }));
  const dialog = screen.getByRole("dialog", { name: "Add Project" });
  fireEvent.click(await within(dialog).findByRole("button", { name: /frontend.*Git/ }));
  await within(dialog).findByText("Git repository");
  expect((within(dialog).getByLabelText("Repository folder") as HTMLInputElement).value).toBe(project.path);
  fireEvent.click(within(dialog).getByRole("button", { name: "Add Project", exact: true }));
  await waitFor(() => expect(api.addProject).toHaveBeenCalledWith(project.path));
});

test("typing filters folders and absolute path prefixes can browse into another directory", async () => {
  const root = "/home/me";
  const work = `${root}/work`;
  const repo = `${work}/frontend`;
  vi.mocked(api.projectFolders).mockImplementation(async (path) => {
    if (path === repo) return { path: repo, roots: [root], parent: work, isRepository: true, folders: [] };
    if (path === work || path === `${work}/`) return {
      path: work, roots: [root], parent: root, isRepository: false,
      folders: [{ name: "frontend", path: repo, isRepository: true }, { name: "backend", path: `${work}/backend`, isRepository: false }],
    };
    return { path: root, roots: [root], parent: null, isRepository: false, folders: [{ name: "work", path: work, isRepository: false }] };
  });
  mount();
  fireEvent.click(await screen.findByRole("button", { name: "Add Project", exact: true }));
  const dialog = within(screen.getByRole("dialog", { name: "Add Project" }));
  const input = dialog.getByLabelText("Repository folder");
  await dialog.findByRole("button", { name: "work" });

  fireEvent.change(input, { target: { value: "missing" } });
  expect(dialog.getByText("No matching folders.")).toBeTruthy();
  fireEvent.change(input, { target: { value: "Wo" } });
  expect(dialog.getByRole("button", { name: "work" })).toBeTruthy();

  fireEvent.change(input, { target: { value: `${work}/FR` } });
  fireEvent.click(await dialog.findByRole("button", { name: /frontend.*Git/ }));
  expect(await dialog.findByText("Git repository")).toBeTruthy();
  expect(input).toHaveProperty("value", repo);
  fireEvent.click(dialog.getByRole("button", { name: "Parent folder" }));
  expect(await dialog.findByRole("button", { name: "backend" })).toBeTruthy();
  expect(input).toHaveProperty("value", work);
});

test("completed reviews remain selectable with a completed appearance and active reviews show statistics", async () => {
  const active = (await api.projectReviews(7))[0];
  vi.mocked(api.projectReviews).mockResolvedValue([active, { ...active, id: 18, headRef: "finished", status: "completed" }]);
  vi.mocked(api.getReview).mockResolvedValue({ ...active, comments: null, reviewedFiles: ["a.ts", "outside-scope.ts"] });
  vi.mocked(api.reviewDiff).mockResolvedValue({ base: "base", head: "head", files: [
    { oldPath: "a.ts", newPath: "a.ts", status: "modified", hunks: [] },
    { oldPath: "b.ts", newPath: "b.ts", status: "modified", hunks: [] },
  ] });
  mount();
  const completed = await screen.findByRole("region", { name: "Review 18 (completed)" });
  expect(completed.classList.contains("is-completed")).toBe(true);
  expect(within(completed).getByRole("link", { name: "Open Review" }).getAttribute("href")).toBe("/reviews?review=18");
  expect(await within(completed).findByText("Completed")).toBeTruthy();
  expect(await within(screen.getByRole("region", { name: "Review 17" })).findByText("1 / 2 (50%)")).toBeTruthy();
  expect(api.reviewDiff).toHaveBeenCalledWith(17);
  expect(api.reviewDiff).toHaveBeenCalledWith(18);
});

test("removing a review confirms, preserves the row on failure and refreshes after success", async () => {
  vi.mocked(api.removeReview).mockRejectedValueOnce(new Error("Write failed")).mockImplementationOnce(async () => {
    vi.mocked(api.projectReviews).mockResolvedValue([]);
  });
  mount();
  fireEvent.click(await screen.findByRole("button", { name: "Remove review 17" }));
  const dialog = screen.getByRole("dialog", { name: "Remove review?" });
  expect(api.removeReview).not.toHaveBeenCalled();
  fireEvent.click(within(dialog).getByRole("button", { name: "Remove" }));
  expect(await within(dialog).findByRole("alert")).toHaveProperty("textContent", "Write failed");
  expect(screen.getByRole("link", { name: "Open Review" })).toBeTruthy();
  fireEvent.click(within(dialog).getByRole("button", { name: "Remove" }));
  await waitFor(() => expect(screen.queryByRole("link", { name: "Open Review" })).toBeNull());
  expect(api.removeReview).toHaveBeenLastCalledWith(17);
});

test("review removal uses a compact confirmation and cancel keeps the review", async () => {
  mount();
  fireEvent.click(await screen.findByRole("button", { name: "Remove review 17" }));
  const dialog = screen.getByRole("dialog", { name: "Remove review?" });
  expect(dialog.classList.contains("modal-sm")).toBe(true);
  expect(within(dialog).queryByRole("button", { name: "Close" })).toBeNull();
  fireEvent.click(within(dialog).getByRole("button", { name: "Cancel" }));
  expect(screen.queryByRole("dialog")).toBeNull();
  expect(api.removeReview).not.toHaveBeenCalled();
  expect(screen.getByRole("link", { name: "Open Review" })).toBeTruthy();
});

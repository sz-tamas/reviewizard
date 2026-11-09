import { beforeEach, expect, test, vi } from "vitest";
import { fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { QueryClientProvider } from "@tanstack/react-query";
import { createQueryClient } from "../app/queryClient";
import { api } from "../api/api";
import { ProjectsPage } from "./ProjectsPage";

vi.mock("../api/api", async (original) => {
  const mod = await original<typeof import("../api/api")>();
  return { ...mod, api: { ...mod.api, projects: vi.fn(), branches: vi.fn(), addProject: vi.fn(), removeProject: vi.fn(), projectFolders: vi.fn(), projectReviews: vi.fn(), commits: vi.fn(), saveProjectReview: vi.fn() } };
});
const project = { id: 7, name: "frontend", path: "/home/me/frontend", reviewCount: 12, openFindings: 3, resolvedFindings: 9, lastActivity: "2026-11-08T12:00:00Z" };
beforeEach(() => {
  vi.clearAllMocks();
  vi.mocked(api.projects).mockResolvedValue([project]);
  vi.mocked(api.branches).mockResolvedValue({ branches: [
    { name: "main", isCurrent: true, isMain: true, isRemote: false, lastCommit: "" },
    { name: "feature/auth", isCurrent: false, isMain: false, isRemote: false, lastCommit: "" },
  ] });
  vi.mocked(api.projectReviews).mockResolvedValue([{ id: 17, repoPath: project.path, headRef: "feature/auth", baseRef: "main", scopeFrom: "all", scopeSide: "index", status: "draft", headSha: "sha", createdAt: "", updatedAt: "", summary: "", comments: [], reviewedFiles: [] }]);
  vi.mocked(api.commits).mockResolvedValue({ commits: [] });
  vi.mocked(api.projectFolders).mockResolvedValue({ path: "/home/me", roots: ["/home/me"], parent: null, isRepository: false, folders: [{ name: "frontend", path: project.path, isRepository: true }] });
});
function mount() {
  const client = createQueryClient();
  render(<QueryClientProvider client={client}><ProjectsPage /></QueryClientProvider>);
  return client;
}

test("cards show persisted counts and carry the selected project and branch to Reviews", async () => {
  mount();
  await screen.findByText(project.path);
  expect(screen.getByText("12 reviews")).toBeTruthy();
  expect((await screen.findByRole("link", { name: "Open Review", exact: true })).getAttribute("href")).toBe("/reviews?review=17");
  expect(screen.getByText(/Staged • from All/)).toBeTruthy();
  fireEvent.change(screen.getByLabelText("Search projects"), { target: { value: "missing" } });
  expect(screen.getByText("No matching projects")).toBeTruthy();
  expect(screen.queryByText(project.path)).toBeNull();
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
  fireEvent.click(screen.getByRole("button", { name: "Add review", exact: true }));
  expect(await screen.findByRole("dialog", { name: "Set up review" })).toBeTruthy();
  expect(api.addProject).toHaveBeenLastCalledWith(project.path);
  client.clear();
});

test("removing a project requires a concrete confirmation and does not delete a review", async () => {
  vi.mocked(api.removeProject).mockResolvedValue(undefined);
  mount();
  await screen.findByText(project.path);
  fireEvent.click(screen.getByLabelText("More actions for frontend"));
  fireEvent.click(screen.getByRole("button", { name: "Remove project", exact: true }));
  expect(api.removeProject).not.toHaveBeenCalled();
  const dialog = screen.getByRole("dialog", { name: "Remove project" });
  expect(within(dialog).getByText(/all existing reviews and findings will be kept/)).toBeTruthy();
  fireEvent.click(within(dialog).getByRole("button", { name: "Remove project", exact: true }));
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

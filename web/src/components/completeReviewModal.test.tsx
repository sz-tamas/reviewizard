import { beforeEach, expect, test, vi } from "vitest";
import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { CompleteReviewModal } from "./CompleteReviewModal";
import { AGENT_PROMPTS } from "../components/review/prompts";
import { LS } from "../preferences/storage";

const stats = { fileCount: 10, reviewedCount: 4, openCommentCount: 7, findingCounts: { bugs: 1, suggestions: 3, other: 3 }, completed: false };
const vars = { origin: "http://localhost", reviewId: 7, headRef: "feature", baseRef: "main", side: "index" as const, fromRef: "abc123" };
const reply = AGENT_PROMPTS.find((p) => p.kind === "reply")!;
beforeEach(() => localStorage.clear());
function mount(onComplete = vi.fn(async (): Promise<string | null> => null)) {
  const onClose = vi.fn();
  render(<CompleteReviewModal repo="repo-a" repoPath="/repo-a" vars={vars} stats={stats} onComplete={onComplete} onClose={onClose} />);
  return { onComplete, onClose };
}

test("the address prompt is copyable without completing the review", async () => {
  let copied = "";
  Object.defineProperty(navigator, "clipboard", { configurable: true, value: { writeText: async (text: string) => { copied = text; } } });
  const { onComplete } = mount();
  expect((screen.getByRole("textbox") as HTMLTextAreaElement).value).toBe(reply.template);
  expect(screen.queryByRole("group", { name: "Review focus" })).toBeNull();
  fireEvent.click(screen.getByRole("button", { name: "Copy" }));
  await screen.findByRole("button", { name: "Copied ✓" });
  expect(copied).toContain("http://localhost/api/reviews/7/export.md?instructions=true");
  expect(onComplete).not.toHaveBeenCalled();
});

test("manual completion shows the three stat cards and saves only after confirmation", async () => {
  const { onComplete, onClose } = mount();
  expect(screen.getByText("4 / 10 (40%)")).toBeTruthy();
  expect(screen.getByText("Findings")).toBeTruthy();
  expect(screen.getByText("7")).toBeTruthy();
  expect(screen.getByText("In progress")).toBeTruthy();
  expect(screen.getByRole("textbox")).toBeTruthy();
  expect(screen.getByRole("region", { name: "Address the review" })).toBeTruthy();
  expect(screen.getByRole("region", { name: "Close manually" })).toBeTruthy();
  expect(screen.queryByRole("group", { name: "Completion method" })).toBeNull();
  expect(onComplete).not.toHaveBeenCalled();
  fireEvent.click(screen.getByRole("button", { name: "Complete review", exact: true }));
  await waitFor(() => expect(onComplete).toHaveBeenCalledTimes(1));
  await waitFor(() => expect(onClose).toHaveBeenCalledTimes(1));
});

test("a failed manual save leaves the dialog open and allows a retry", async () => {
  const { onComplete, onClose } = mount(vi.fn(async () => "Status could not be saved"));
  fireEvent.click(screen.getByRole("button", { name: "Complete review", exact: true }));
  expect((await screen.findByRole("alert")).textContent).toBe("Status could not be saved");
  expect(onComplete).toHaveBeenCalledTimes(1);
  expect(onClose).not.toHaveBeenCalled();
  expect((screen.getByRole("button", { name: "Complete review", exact: true }) as HTMLButtonElement).disabled).toBe(false);
});

test("both completion options remain visible while editing the address prompt", () => {
  mount();
  fireEvent.change(screen.getByRole("textbox"), { target: { value: "address draft" } });
  expect((screen.getByRole("textbox") as HTMLTextAreaElement).value).toBe("address draft");
  expect(localStorage.getItem(LS.agentPromptsByRepo)).toBeNull();
});

test("existing reply overrides move here and stay independent of lens templates", () => {
  localStorage.setItem(LS.agentPromptsByRepo, JSON.stringify({ "repo-a": { reply: "custom reply", review: "custom lens" } }));
  const { onComplete } = mount();
  expect((screen.getByRole("textbox") as HTMLTextAreaElement).value).toBe("custom reply");
  fireEvent.click(screen.getByRole("button", { name: "Reset", exact: true }));
  expect((screen.getByRole("textbox") as HTMLTextAreaElement).value).toBe(reply.template);
  expect(JSON.parse(localStorage.getItem(LS.agentPromptsByRepo)!)).toEqual({ "repo-a": { review: "custom lens" } });
  fireEvent.change(screen.getByRole("textbox"), { target: { value: "new reply" } });
  fireEvent.click(screen.getByRole("button", { name: "Save", exact: true }));
  expect(JSON.parse(localStorage.getItem(LS.agentPromptsByRepo)!)).toEqual({ "repo-a": { review: "custom lens", reply: "new reply" } });
  expect(onComplete).not.toHaveBeenCalled();
});

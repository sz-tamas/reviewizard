import { beforeEach, expect, test, vi } from "vitest";
import { fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { CompleteReviewModal } from "./CompleteReviewModal";
import { LS } from "../preferences/storage";

const stats = { fileCount: 10, reviewedCount: 4, openCommentCount: 7, findingCounts: { bugs: 1, suggestions: 3, other: 3 }, completed: false };
const vars = { origin: "http://localhost", reviewId: 7, headRef: "feature", baseRef: "main", side: "index" as const, fromRef: "abc123" };
beforeEach(() => localStorage.clear());
function mount(onComplete = vi.fn(async (): Promise<string | null> => null)) {
  const onClose = vi.fn();
  render(<CompleteReviewModal repoPath="/repo-a" vars={vars} stats={stats} onComplete={onComplete} onClose={onClose} />);
  return { onComplete, onClose };
}

test("the address prompt is copyable without completing the review", async () => {
  let copied = "";
  Object.defineProperty(navigator, "clipboard", { configurable: true, value: { writeText: async (text: string) => { copied = text; } } });
  const { onComplete } = mount();
  const preview = screen.getByRole("textbox") as HTMLTextAreaElement;
  expect(preview.readOnly).toBe(true);
  expect(screen.queryByRole("group", { name: "Review focus" })).toBeNull();
  const handoff = within(screen.getByRole("region", { name: "Address the review" }));
  const format = handoff.getByRole("combobox", { name: "Agent / format" }) as HTMLSelectElement;
  expect(format.disabled).toBe(true);
  expect(format.value).toBe("Generic agent (Markdown)");
  fireEvent.click(handoff.getByRole("button", { name: "Copy" }));
  await screen.findByRole("button", { name: "Copied ✓" });
  expect(copied).toBe(preview.value);
  expect(copied).toContain("http://localhost/api/reviews/7/export.md?instructions=true");
  expect(copied).toContain("Fix and verify findings");
  expect(copied).toContain("Leave disputed findings, clarification questions and unverified fixes open");
  expect(copied).toContain("same machine as reviewizard");
  expect(copied).toContain("your tool’s approval mechanism");
  expect(copied).toContain("target repository: /repo-a");
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
  const manual = within(screen.getByRole("region", { name: "Close manually" }));
  expect(screen.queryByRole("button", { name: "Cancel" })).toBeNull();
  expect(screen.queryByRole("group", { name: "Completion method" })).toBeNull();
  expect(onComplete).not.toHaveBeenCalled();
  fireEvent.click(manual.getByRole("button", { name: "Complete review", exact: true }));
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

test("the generated handoff ignores saved overrides and offers no editing actions", () => {
  const overrides = JSON.stringify({ "repo-a": { reply: "custom reply", review: "custom lens" } });
  localStorage.setItem(LS.agentPromptsByRepo, overrides);
  mount();
  const preview = screen.getByRole("textbox") as HTMLTextAreaElement;
  expect(preview.value).toContain("target repository: /repo-a");
  expect(preview.value).not.toContain("custom reply");
  expect(screen.queryByRole("button", { name: "Reset", exact: true })).toBeNull();
  expect(screen.queryByRole("button", { name: "Save", exact: true })).toBeNull();
  expect(localStorage.getItem(LS.agentPromptsByRepo)).toBe(overrides);
});

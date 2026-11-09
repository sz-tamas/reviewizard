import { expect, test } from "vitest";
import { fireEvent, render, screen } from "@testing-library/react";

import { TopBar } from "./TopBar";
import type { Selection, TopBarStatus } from "./TopBar";
import type { Review } from "../api/types";

// The "from" picker used to leave a reviewer guessing whether the picked commit's
// own changes were in the diff, and the file count had nothing to explain itself
// against — so the readout's wording is the fix and worth pinning.

const selection: Selection = {
  repo: "proj",
  repoOptions: [],
  onRepoChange: () => {},
  head: "feature",
  headOptions: [],
  onHeadChange: () => {},
  base: "main",
  baseOptions: [],
  onBaseChange: () => {},
  baseRelevant: true,
  from: "all",
  fromOptions: [],
  onFromChange: () => {},
  commitCount: 3,
  headIsCurrent: true,
  baseIsHead: false,
  side: "head",
  onSideChange: () => {},
  loading: false,
  onReload: () => {},
};

const review = { id: 1, headRef: "feature", headSha: "9f8e7d6c" } as Review;

const status: TopBarStatus = {
  review,
  shortSha: "9f8e7d6",
  baseSha: "1a2b3c4d5e",
  fileCount: 5,
  stat: { added: 10, removed: 2 },
  openCommentCount: 0,
  canReset: false,
};

const actions = {
  onShowPrompts: () => {},
  onShowExport: () => {},
  onReset: () => {},
};

test("the overview describes the saved scope and actual review progress", () => {
  render(<TopBar selection={{ ...selection, side: "index", from: "abc123456" }} actions={actions} status={{ ...status, reviewedCount: 2, openCommentCount: 7, findingCounts: { bugs: 1, suggestions: 3, other: 3 } }} />);
  expect(screen.getByRole("heading", { name: /proj.*feature/ })).toBeTruthy();
  expect(screen.getByText(/Review: Staged/)).toBeTruthy();
  expect(screen.getByText(/From abc1234 \(inclusive\)/)).toBeTruthy();
  expect(screen.getByText("2 / 5 (40%)")).toBeTruthy();
  expect(screen.getByRole("progressbar", { name: "Files reviewed" }).getAttribute("aria-valuenow")).toBe("2");
  expect(screen.queryByLabelText("head branch")).toBeNull();
  expect(screen.queryByLabelText("diff side")).toBeNull();
});


test("review actions preserve the existing prompts, export, reset and reload", async () => {
  const calls: string[] = [];
  render(<TopBar selection={{ ...selection, onReload: () => calls.push("reload") }} actions={{ onShowPrompts: () => calls.push("prompts"), onShowExport: () => calls.push("export"), onReset: () => calls.push("reset"), onComplete: () => calls.push("complete") }} status={{ ...status, canReset: true }} />);
  fireEvent.click(screen.getByRole("button", { name: "AI Review Handoff" }));
  fireEvent.click(screen.getByRole("button", { name: "Complete Review" }));
  for (const name of ["Export review", "Reset review", "Reload"]) {
    fireEvent.keyDown(screen.getByRole("button", { name: "More review actions" }), { key: "ArrowDown" });
    fireEvent.click(await screen.findByRole("menuitem", { name }));
  }
  expect(screen.queryByRole("button", { name: "Settings" })).toBeNull();
  expect(screen.queryByRole("button", { name: "Help" })).toBeNull();
  expect(calls).toEqual(["prompts", "complete", "export", "reset", "reload"]);
  expect(screen.queryByRole("menu")).toBeNull();
});

test("completed reviews can be reopened and the header logo returns home", () => {
  render(<TopBar selection={selection} actions={actions} status={{ ...status, review: { ...review, status: "completed" } }} />);
  expect(screen.getByText("Completed")).toBeTruthy();
  expect(screen.getByRole("button", { name: "Reopen Review" })).toBeTruthy();
  expect(screen.getByRole("link", { name: "reviewizard home" }).getAttribute("href")).toBe("/");
  expect(screen.queryByRole("navigation", { name: "Main navigation" })).toBeNull();
});

test("codebase overview identifies the entire codebase without a comparison base", () => {
  render(<TopBar selection={selection} actions={actions} status={{ ...status, review: { ...review, scopeMode: "codebase", baseRef: "" } }} />);
  expect(screen.getByText(/Scope: Entire codebase/)).toBeTruthy();
  expect(screen.queryByText(/Base:/)).toBeNull();
});

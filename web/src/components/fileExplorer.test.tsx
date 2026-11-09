import { expect, test } from "vitest";
import { fireEvent, render, screen } from "@testing-library/react";
import { FileExplorer } from "./FileExplorer";
import type { Comment, FileDiff } from "../api/types";

const files: FileDiff[] = [
  { oldPath: "a.ts", newPath: "a.ts", status: "modified", hunks: [] },
  { oldPath: "b.ts", newPath: "b.ts", status: "modified", hunks: [] },
];
function mount() {
  render(<FileExplorer files={files} comments={[{ id: 1, filePath: "b.ts", resolved: false } as Comment]} reviewed={new Set(["a.ts"])} selected={null} onSelect={() => {}} onToggleReviewed={() => {}} onToggleFolder={() => {}} onAddFile={() => {}} onCollapse={() => {}} scopeSummary="feature → main, Staged, from All" />);
}

test("changed-file filters combine actual reviewed marks and open findings", () => {
  mount();
  fireEvent.click(screen.getByRole("button", { name: /Unreviewed 1/ }));
  expect(screen.queryByRole("button", { name: /M a.ts/ })).toBeNull();
  expect(screen.getByTitle("b.ts")).toBeTruthy();
  fireEvent.click(screen.getByRole("button", { name: /Flagged 1/ }));
  expect(screen.getByTitle("b.ts")).toBeTruthy();
  fireEvent.click(screen.getByRole("button", { name: /All 2/ }));
  expect(screen.getByTitle("a.ts")).toBeTruthy();
});

test("the scope tab describes the saved comparison without editing it", () => {
  mount();
  fireEvent.click(screen.getByRole("button", { name: "Review Scope" }));
  expect(screen.getByText("feature → main, Staged, from All")).toBeTruthy();
  expect(screen.queryByTitle("a.ts")).toBeNull();
  fireEvent.click(screen.getByRole("button", { name: "Files" }));
  expect(screen.getByTitle("a.ts")).toBeTruthy();
});

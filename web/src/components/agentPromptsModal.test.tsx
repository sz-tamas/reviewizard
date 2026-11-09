import { beforeEach, expect, test } from "vitest";
import { fireEvent, render, screen } from "@testing-library/react";
import { AgentPromptsModal } from "./AgentPromptsModal";
import { LS } from "../preferences/storage";

const vars = { origin: "http://localhost", reviewId: 7, headRef: "feature", baseRef: "main", side: "index" as const, fromRef: "abc123" };
function open() { render(<AgentPromptsModal repo="repo-a" repoPath="/work/app" vars={vars} onClose={() => {}} />); }
function output() { return screen.getByRole("textbox", { name: "Generated handoff prompt" }) as HTMLTextAreaElement; }
function generate() { fireEvent.click(screen.getByRole("button", { name: "Generate Handoff Prompt" })); }
beforeEach(() => localStorage.clear());

test("handoff starts with combinable lenses and an empty output", () => {
  open();
  expect(screen.getAllByRole("checkbox")).toHaveLength(6);
  expect((screen.getByRole("checkbox", { name: "Correctness" }) as HTMLInputElement).checked).toBe(true);
  expect((screen.getByRole("checkbox", { name: "Security" }) as HTMLInputElement).checked).toBe(true);
  expect((screen.getByRole("radio", { name: "Medium" }) as HTMLInputElement).checked).toBe(true);
  expect((screen.getByLabelText("Agent / format") as HTMLSelectElement).value).toBe("codex-cli");
  expect(screen.queryByRole("group", { name: "Review focus" })).toBeNull();
  expect(output().value).toBe("");
  expect((screen.getByRole("button", { name: "Copy", exact: true }) as HTMLButtonElement).disabled).toBe(true);
});

test("Generate combines checked lenses, severity and format into the output", () => {
  open();
  fireEvent.click(screen.getByRole("checkbox", { name: "Security" }));
  fireEvent.click(screen.getByRole("checkbox", { name: "Performance" }));
  fireEvent.click(screen.getByRole("checkbox", { name: "Maintainability" }));
  fireEvent.click(screen.getByRole("radio", { name: "High" }));
  fireEvent.change(screen.getByLabelText("Agent / format"), { target: { value: "claude-code" } });
  expect(output().value).toBe("");
  generate();
  expect(output().value).toContain("# Claude Code review handoff");
  for (const lens of ["Correctness", "Performance", "Maintainability"]) expect(output().value).toContain(`- ${lens}:`);
  expect(output().value).not.toContain("- Security:");
  expect(output().value).toContain("## Minimum severity: High");
  expect(output().value).toContain('"side":"index"');
  expect(output().value).toContain("/api/reviews/7/diff");
});

test("changing options preserves edits until Generate and prevents copying stale settings", () => {
  open(); generate();
  fireEvent.change(output(), { target: { value: "My edited handoff" } });
  fireEvent.click(screen.getByRole("checkbox", { name: "Tests" }));
  expect(output().value).toBe("My edited handoff");
  expect(screen.getByRole("status").textContent).toContain("Options changed");
  expect((screen.getByRole("button", { name: "Copy", exact: true }) as HTMLButtonElement).disabled).toBe(true);
  generate();
  expect(output().value).toContain("- Tests:");
  expect((screen.getByRole("button", { name: "Copy", exact: true }) as HTMLButtonElement).disabled).toBe(false);
});

test("the generated result can be edited and copied exactly as shown", async () => {
  let copied = "";
  Object.defineProperty(navigator, "clipboard", { configurable: true, value: { writeText: async (text: string) => { copied = text; } } });
  open(); generate();
  fireEvent.change(output(), { target: { value: "edited output with {{literal}}" } });
  fireEvent.click(screen.getByRole("button", { name: "Copy", exact: true }));
  await screen.findByRole("button", { name: "Copied ✓" });
  expect(copied).toBe("edited output with {{literal}}");
});

test("JSON selection produces structured output after regeneration", () => {
  open();
  fireEvent.change(screen.getByLabelText("Agent / format"), { target: { value: "json" } });
  generate();
  expect(JSON.parse(output().value).scope).toMatchObject({ reviewId: 7, side: "index", from: "abc123" });
});

test("at least one lens is required", () => {
  open();
  fireEvent.click(screen.getByRole("checkbox", { name: "Correctness" }));
  fireEvent.click(screen.getByRole("checkbox", { name: "Security" }));
  expect((screen.getByRole("button", { name: "Generate Handoff Prompt" }) as HTMLButtonElement).disabled).toBe(true);
  expect(screen.getByText("Select at least one review lens.")).toBeTruthy();
});

test("retired single-lens templates and the reply override are not overwritten or mixed into a combination", () => {
  const saved = { "repo-a": { review: "old correctness only", "review:security": "old security only", reply: "existing reply" } };
  localStorage.setItem(LS.agentPromptsByRepo, JSON.stringify(saved));
  open(); generate();
  expect(output().value).not.toContain("old correctness only");
  expect(output().value).not.toContain("old security only");
  expect(JSON.parse(localStorage.getItem(LS.agentPromptsByRepo)!)).toEqual(saved);
});

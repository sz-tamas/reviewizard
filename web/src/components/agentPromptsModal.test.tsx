import { beforeEach, expect, test } from "vitest";
import { fireEvent, render, screen } from "@testing-library/react";
import { AgentPromptsModal } from "./AgentPromptsModal";
import { LS } from "../preferences/storage";

const vars = { origin: "http://localhost", reviewId: 7, headRef: "feature", baseRef: "main", side: "index" as const, fromRef: "abc123" };
function open() { render(<AgentPromptsModal repo="repo-a" repoPath="/work/app" vars={vars} onClose={() => {}} />); }
function output() { return screen.getByRole("textbox", { name: "Generated handoff prompt" }) as HTMLTextAreaElement; }
beforeEach(() => localStorage.clear());

test("handoff starts with combinable lenses and a copyable default prompt", () => {
  open();
  expect(screen.getAllByRole("checkbox")).toHaveLength(6);
  expect((screen.getByRole("checkbox", { name: "Correctness" }) as HTMLInputElement).checked).toBe(true);
  expect((screen.getByRole("checkbox", { name: "Security" }) as HTMLInputElement).checked).toBe(true);
  expect((screen.getByRole("radio", { name: "Medium" }) as HTMLInputElement).checked).toBe(true);
  expect((screen.getByLabelText("Agent / format") as HTMLSelectElement).value).toBe("markdown");
  expect(screen.queryByRole("group", { name: "Review focus" })).toBeNull();
  expect(output().value).toContain("# Generic agent (Markdown) review handoff");
  expect(screen.getAllByRole("button", { name: "Copy", exact: true })).toHaveLength(1);
  expect((screen.getByRole("button", { name: "Copy", exact: true }) as HTMLButtonElement).disabled).toBe(false);
});

test("The prompt automatically combines checked lenses, severity and format into the output", () => {
  open();
  fireEvent.click(screen.getByRole("checkbox", { name: "Security" }));
  fireEvent.click(screen.getByRole("checkbox", { name: "Performance" }));
  fireEvent.click(screen.getByRole("checkbox", { name: "Maintainability" }));
  fireEvent.click(screen.getByRole("radio", { name: "High" }));
  fireEvent.change(screen.getByLabelText("Agent / format"), { target: { value: "claude-code" } });
  expect(output().value).toContain("# Claude Code review handoff");
  for (const lens of ["Correctness", "Performance", "Maintainability"]) expect(output().value).toContain(`- ${lens}:`);
  expect(output().value).not.toContain("- Security:");
  expect(output().value).toContain("## Minimum severity: High");
  expect(output().value).toContain('"side":"index"');
  expect(output().value).toContain("/api/reviews/7/diff");
});

test("changing options replaces edited text with the updated prompt", () => {
  open();
  fireEvent.change(output(), { target: { value: "My edited handoff" } });
  fireEvent.click(screen.getByRole("checkbox", { name: "Tests" }));
  expect(output().value).toContain("- Tests:");
  expect(output().value).not.toContain("My edited handoff");
  expect((screen.getByRole("button", { name: "Copy", exact: true }) as HTMLButtonElement).disabled).toBe(false);
});

test("the generated result can be edited and copied exactly as shown", async () => {
  let copied = "";
  Object.defineProperty(navigator, "clipboard", { configurable: true, value: { writeText: async (text: string) => { copied = text; } } });
  open();
  fireEvent.change(output(), { target: { value: "edited output with {{literal}}" } });
  fireEvent.click(screen.getByRole("button", { name: "Copy", exact: true }));
  await screen.findByRole("button", { name: "Copied ✓" });
  expect(copied).toBe("edited output with {{literal}}");
});

test("JSON selection produces structured output immediately", () => {
  open();
  fireEvent.change(screen.getByLabelText("Agent / format"), { target: { value: "json" } });
  expect(JSON.parse(output().value).scope).toMatchObject({ reviewId: 7, side: "index", from: "abc123" });
});

test("at least one lens is required", () => {
  open();
  fireEvent.click(screen.getByRole("checkbox", { name: "Correctness" }));
  fireEvent.click(screen.getByRole("checkbox", { name: "Security" }));
  expect(output().value).toBe("");
  expect((screen.getByRole("button", { name: "Copy", exact: true }) as HTMLButtonElement).disabled).toBe(true);
  expect(screen.getByText("Select at least one review lens.")).toBeTruthy();
});

test("retired single-lens templates and the reply override are not overwritten or mixed into a combination", () => {
  const saved = { "repo-a": { review: "old correctness only", "review:security": "old security only", reply: "existing reply" } };
  localStorage.setItem(LS.agentPromptsByRepo, JSON.stringify(saved));
  open();
  expect(output().value).not.toContain("old correctness only");
  expect(output().value).not.toContain("old security only");
  expect(JSON.parse(localStorage.getItem(LS.agentPromptsByRepo)!)).toEqual(saved);
});

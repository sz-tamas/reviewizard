import { expect, test } from "vitest";
import { AGENT_PROMPTS, DEFAULT_HANDOFF_OPTIONS, HANDOFF_FORMATS, PROMPT_PLACEHOLDERS, REVIEW_LENSES, generateHandoffPrompt, renderPrompt } from "./prompts";

const vars = { origin: "http://127.0.0.1:7777", reviewId: 7, headRef: "feat/x", baseRef: "main", author: "review-agent", repoPath: "/projects/my repo", side: "index" as const, fromRef: "abc123" };

test("reply placeholders still resolve and unknown properties remain literal", () => {
  for (const name of PROMPT_PLACEHOLDERS) expect(renderPrompt(`{{${name}}}`, vars)).not.toBe(`{{${name}}}`);
  expect(renderPrompt("{{reviewId}} {{reviewId}} {{constructor}} {{orgin}}", vars)).toBe("7 7 {{constructor}} {{orgin}}");
  expect(renderPrompt(AGENT_PROMPTS[0].template, vars)).toContain("/api/reviews/7/export.md?instructions=true");
});

test("selected lenses combine into one assignment with no unselected briefs", () => {
  const text = generateHandoffPrompt(vars, vars.repoPath, { ...DEFAULT_HANDOFF_OPTIONS, lenses: ["correctness", "performance", "tests"] });
  for (const label of ["Correctness", "Performance", "Tests"]) expect(text).toContain(`- ${label}:`);
  for (const label of ["Security", "Design", "Maintainability"]) expect(text).not.toContain(`- ${label}:`);
  expect(text).toContain("one combined review");
  expect(text).toContain("Deduplicate overlapping issues");
  expect(text).not.toContain("Use one specific lens");
});

test("all six lenses can be selected without duplicate briefs", () => {
  const text = generateHandoffPrompt(vars, vars.repoPath, { ...DEFAULT_HANDOFF_OPTIONS, lenses: [...REVIEW_LENSES.map((l) => l.id), "security"] });
  for (const lens of REVIEW_LENSES) expect(text.split(`- ${lens.label}:`).length).toBe(2);
});

test("minimum severity changes the allowed reporting threshold without changing API fields", () => {
  for (const [minimumSeverity, accepted] of [["low", ["low", "medium", "high"]], ["medium", ["medium", "high"]], ["high", ["high"]]] as const) {
    const parsed = JSON.parse(generateHandoffPrompt(vars, vars.repoPath, { ...DEFAULT_HANDOFF_OPTIONS, minimumSeverity, format: "json" }));
    expect(parsed.minimumSeverity).toBe(minimumSeverity);
    expect(parsed.acceptedSeverities).toEqual(accepted);
    expect(parsed.reporting.example).not.toHaveProperty("severity");
    expect(parsed.reporting.example.body).toContain(`[${minimumSeverity[0].toUpperCase() + minimumSeverity.slice(1)}]`);
  }
});

test("every format carries the exact saved review scope and real comment source side", () => {
  for (const format of HANDOFF_FORMATS) {
    const text = generateHandoffPrompt(vars, vars.repoPath, { ...DEFAULT_HANDOFF_OPTIONS, format: format.value });
    expect(text).toContain("http://127.0.0.1:7777/api/reviews/7/diff");
    expect(text).toContain("abc123");
    expect(text).toContain("feat/x");
    expect(text).toContain('"side"');
    expect(text).toContain('"index"');
    expect(text).not.toContain("{{");
    if (format.value !== "json") expect(text).toContain(`# ${format.label} review handoff`);
  }
});

test("JSON is a structured handoff with actionable reporting instructions", () => {
  const parsed = JSON.parse(generateHandoffPrompt(vars, vars.repoPath, { ...DEFAULT_HANDOFF_OPTIONS, format: "json", lenses: ["design", "maintainability"] }));
  expect(parsed.repository).toBe(vars.repoPath);
  expect(parsed.scope).toMatchObject({ reviewId: 7, headRef: "feat/x", baseRef: "main", from: "abc123", startingCommitInclusive: true, side: "index" });
  expect(parsed.lenses.map((l: { id: string }) => l.id)).toEqual(["design", "maintainability"]);
  expect(parsed.reporting).toMatchObject({ method: "POST", url: "http://127.0.0.1:7777/api/reviews/7/comments", example: { side: "index", author: "review-agent" } });
});

test("omitted legacy scope fields default explicitly to the committed whole branch", () => {
  const parsed = JSON.parse(generateHandoffPrompt({ origin: vars.origin, reviewId: 7, headRef: "f", baseRef: "main" }, "/repo", { ...DEFAULT_HANDOFF_OPTIONS, format: "json" }));
  expect(parsed.scope).toMatchObject({ side: "head", from: "all", startingCommitInclusive: false });
});

test("no selected lenses cannot produce an assignment", () => {
  expect(() => generateHandoffPrompt(vars, vars.repoPath, { ...DEFAULT_HANDOFF_OPTIONS, lenses: [] })).toThrow("Select at least one");
});

test("generated JSON safely preserves unusual repository and ref strings", () => {
  const path = "/projects/quote' and \"double\"/repo";
  const parsed = JSON.parse(generateHandoffPrompt({ ...vars, headRef: "feature's-work" }, path, { ...DEFAULT_HANDOFF_OPTIONS, format: "json" }));
  expect(parsed.repository).toBe(path);
  expect(parsed.scope.headRef).toBe("feature's-work");
});

test("every handoff explicitly focuses the target folder in a multi-repository workspace", () => {
  const target = "/Users/me/Work/selected repo";
  for (const format of HANDOFF_FORMATS) {
    const output = generateHandoffPrompt(vars, target, { ...DEFAULT_HANDOFF_OPTIONS, format: format.value });
    const instruction = format.value === "json" ? JSON.parse(output).repositoryFocus : output;
    expect(instruction).toContain(`Focus this review only on the repository at ${JSON.stringify(target)}`);
    expect(instruction).toContain("use this folder as the working directory for all local inspection and Git commands");
    expect(instruction).toContain("do not review sibling repositories");
  }
});

test("every codebase handoff covers unchanged files with the saved source and reporting API", () => {
  for (const format of HANDOFF_FORMATS) {
    const output = generateHandoffPrompt({ ...vars, mode: "codebase", baseRef: "", fromRef: "all" }, vars.repoPath, { ...DEFAULT_HANDOFF_OPTIONS, format: format.value, lenses: ["correctness", "tests"] });
    expect(output).toContain("entire codebase");
    expect(output).toContain("including unchanged files");
    expect(output).toContain("/api/reviews/7/diff");
    expect(output).toContain("/api/reviews/7/comments");
    expect(output).not.toContain("Trace changed behavior");
    expect(output).not.toContain("Map changed behaviors");
    if (format.value === "json") {
      const parsed = JSON.parse(output);
      expect(parsed.scope).toMatchObject({ mode: "codebase", side: "index", baseRef: "", from: "all" });
      expect(parsed.reporting.example.side).toBe("index");
    } else {
      expect(output).toContain("Scope: Entire codebase");
      expect(output).not.toContain("whole branch");
    }
  }
});

test("handoffs diagnose sandbox access before declaring the local server unavailable", () => {
  for (const format of HANDOFF_FORMATS) {
    const output = generateHandoffPrompt(vars, vars.repoPath, { ...DEFAULT_HANDOFF_OPTIONS, format: format.value });
    const guidance = format.value === "json" ? JSON.parse(output).connectionInstructions : output;
    expect(guidance).toContain("curl exit 7");
    expect(guidance).toContain("same machine as reviewizard");
    expect(guidance).toContain("your tool’s approval mechanism");
    expect(guidance).not.toContain("sandbox_permissions");
    expect(output).not.toContain("If this fails, stop");
  }
});

test("the default format generates a generic local agent prompt", () => {
  const output = generateHandoffPrompt(vars, vars.repoPath, DEFAULT_HANDOFF_OPTIONS);
  expect(output).toContain("# Generic agent (Markdown) review handoff");
  expect(output).not.toMatch(/Codex|Claude/i);
});

test("the Codex format explicitly supports desktop and CLI", () => {
  const output = generateHandoffPrompt(vars, vars.repoPath, { ...DEFAULT_HANDOFF_OPTIONS, format: "codex-cli" });
  expect(output).toContain("Codex (desktop / CLI)");
  expect(output).toContain("local Codex desktop app or Codex CLI session");
});

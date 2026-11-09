import { useState } from "react";
import { AGENT_PROMPTS, PROMPT_PLACEHOLDERS, renderPrompt } from "./review/prompts";
import type { ReviewVars } from "./review/prompts";
import { clearPromptOverride, readPromptOverride, writePromptOverride } from "../preferences/storage";
import { CopyButton } from "./CopyButton";

export interface ReplyPromptEditorProps {
  repo: string;
  repoPath?: string;
  vars: ReviewVars;
}

export function ReplyPromptEditor({ repo, repoPath = repo, vars }: ReplyPromptEditorProps) {
  const template = AGENT_PROMPTS[0];
  const [saved, setSaved] = useState(() => readPromptOverride(repo, "reply"));
  const [draft, setDraft] = useState(() => saved ?? template.template);
  const dirty = draft !== (saved ?? template.template);
  const customized = saved !== null || draft !== template.template;
  const promptVars = { ...vars, author: template.author, repoPath };
  function save() {
    writePromptOverride(repo, "reply", draft);
    setSaved(draft);
  }
  function reset() {
    clearPromptOverride(repo, "reply");
    setSaved(null);
    setDraft(template.template);
  }
  return <div className="agent-prompt-editor">
    <div className="modal-subhead prompt-editor-controls">
      <span>Address the review</span><span className="spacer" />
      <CopyButton className="btn copy-btn" text={() => renderPrompt(draft, promptVars)} idleLabel="Copy" title="Copy with the placeholders filled in for this review" />
    </div>
    <textarea className="prompt-editor" aria-label="Address the review prompt" spellCheck={false} value={draft} onChange={(e) => setDraft(e.target.value)} data-autofocus />
    <div className="modal-foot">
      <p className="prompt-hint" title={PROMPT_PLACEHOLDERS.map((name) => `{{${name}}} → ${renderPrompt(`{{${name}}}`, promptVars)}`).join("\n")}>
        Filled in on copy: {PROMPT_PLACEHOLDERS.map((name) => `{{${name}}}`).join(" ")}
      </p>
      <span className="spacer" />
      {dirty && <span className="prompt-dirty">Unsaved changes</span>}
      <button className="btn" onClick={reset} disabled={!customized}>Reset</button>
      <button className="btn btn-primary" onClick={save} disabled={!dirty || !draft.trim()}>Save</button>
    </div>
  </div>;
}

import { useId, useState } from "react";
import { DEFAULT_HANDOFF_OPTIONS, HANDOFF_FORMATS, REVIEW_LENSES, generateHandoffPrompt } from "./review/prompts";
import type { HandoffFormat, HandoffOptions, MinimumSeverity, ReviewLens, ReviewVars } from "./review/prompts";
import { CopyButton } from "./CopyButton";
import { Modal } from "./Modal";

export function AgentPromptsModal({ repo, repoPath = repo, vars, onClose }: {
  repo: string;
  repoPath?: string;
  vars: ReviewVars;
  onClose: () => void;
}) {
  const severityId = useId();
  const [options, setOptions] = useState<HandoffOptions>(() => ({ ...DEFAULT_HANDOFF_OPTIONS, lenses: [...DEFAULT_HANDOFF_OPTIONS.lenses] }));
  const [edit, setEdit] = useState<{ signature: string; text: string }>();
  const signature = JSON.stringify({ options, repoPath, ...vars });
  const hasLenses = options.lenses.length > 0;
  const prompt = !hasLenses ? "" : edit?.signature === signature ? edit.text : generateHandoffPrompt(vars, repoPath, options);
  const outputId = `${severityId}-output`;

  function updateOptions(update: (previous: HandoffOptions) => HandoffOptions) {
    setEdit(undefined);
    setOptions(update);
  }

  function toggleLens(id: ReviewLens) {
    updateOptions((prev) => ({
      ...prev,
      lenses: REVIEW_LENSES.filter((lens) => lens.id === id ? !prev.lenses.includes(id) : prev.lenses.includes(lens.id)).map((lens) => lens.id),
    }));
  }

  return <Modal title="AI Review Handoff" onClose={onClose} className="handoff-generator-modal">
    <div className="handoff-generator-body">
      <p className="handoff-hint">Select one or more lenses to combine in a single review.</p>
      <fieldset className="handoff-fieldset">
        <legend>Review lenses</legend>
        <div className="handoff-lenses">
          {REVIEW_LENSES.map((lens) => <label key={lens.id} className={`handoff-lens${options.lenses.includes(lens.id) ? " is-selected" : ""}`}>
            <input type="checkbox" checked={options.lenses.includes(lens.id)} onChange={() => toggleLens(lens.id)} aria-label={lens.label} />
            <span><strong>{lens.label}</strong><small>{lens.description}</small></span>
          </label>)}
        </div>
        {!options.lenses.length && <p className="handoff-hint">Select at least one review lens.</p>}
      </fieldset>
      <fieldset className="handoff-fieldset">
        <legend>Minimum severity to report</legend>
        <div className="handoff-severity">
          {(["low", "medium", "high"] as MinimumSeverity[]).map((severity) => <label key={severity}>
            <input type="radio" name={severityId} value={severity} checked={options.minimumSeverity === severity} onChange={() => updateOptions((prev) => ({ ...prev, minimumSeverity: severity }))} />
            {severity[0].toUpperCase() + severity.slice(1)}
          </label>)}
        </div>
      </fieldset>
      <label className="handoff-format">Agent / format
        <select aria-label="Agent / format" value={options.format} onChange={(e) => updateOptions((prev) => ({ ...prev, format: e.target.value as HandoffFormat }))}>
          {HANDOFF_FORMATS.map((format) => <option key={format.value} value={format.value}>{format.label}</option>)}
        </select>
      </label>
      <div className="handoff-output">
        <label htmlFor={outputId}>Generated handoff prompt</label>
        <textarea id={outputId} className="prompt-editor handoff-output-text" spellCheck={false} value={prompt} readOnly={!hasLenses} onChange={(e) => setEdit({ signature, text: e.target.value })} />
        {hasLenses && <p className="handoff-hint">You can edit the prompt before copying. Changing the review options regenerates it.</p>}
      </div>
    </div>
    <div className="modal-foot handoff-footer">
      {prompt.trim()
        ? <CopyButton key={signature} className="project-primary" text={prompt} idleLabel="Copy" />
        : <button className="project-primary" disabled>Copy</button>}
    </div>
  </Modal>;
}

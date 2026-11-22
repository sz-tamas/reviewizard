import { useId, useState } from "react";
import { FileText } from "lucide-react";
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
  const [prompt, setPrompt] = useState("");
  const [generatedSignature, setGeneratedSignature] = useState("");
  const [error, setError] = useState("");
  const signature = JSON.stringify({ options, repoPath, ...vars });
  const outdated = !!generatedSignature && generatedSignature !== signature;
  const canCopy = !!prompt.trim() && generatedSignature === signature;

  function toggleLens(id: ReviewLens) {
    setOptions((prev) => ({
      ...prev,
      lenses: REVIEW_LENSES.filter((lens) => lens.id === id ? !prev.lenses.includes(id) : prev.lenses.includes(lens.id)).map((lens) => lens.id),
    }));
  }
  function generate() {
    try {
      setPrompt(generateHandoffPrompt(vars, repoPath, options));
      setGeneratedSignature(signature);
      setError("");
    } catch (e) { setError((e as Error).message); }
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
            <input type="radio" name={severityId} value={severity} checked={options.minimumSeverity === severity} onChange={() => setOptions((prev) => ({ ...prev, minimumSeverity: severity }))} />
            {severity[0].toUpperCase() + severity.slice(1)}
          </label>)}
        </div>
      </fieldset>
      <label className="handoff-format">Agent / format
        <select aria-label="Agent / format" value={options.format} onChange={(e) => setOptions((prev) => ({ ...prev, format: e.target.value as HandoffFormat }))}>
          {HANDOFF_FORMATS.map((format) => <option key={format.value} value={format.value}>{format.label}</option>)}
        </select>
      </label>
      <button className="project-primary handoff-generate" onClick={generate} disabled={!options.lenses.length}><FileText size={16} />Generate Handoff Prompt</button>
      <div className="handoff-output">
        <div className="handoff-output-head">
          <label htmlFor={`${severityId}-output`}>Handoff prompt</label>
          {canCopy ? <CopyButton text={prompt} idleLabel="Copy" /> : <button className="btn" disabled>Copy</button>}
        </div>
        <textarea id={`${severityId}-output`} aria-label="Generated handoff prompt" className="prompt-editor handoff-output-text" spellCheck={false} value={prompt} readOnly={!generatedSignature} onChange={(e) => setPrompt(e.target.value)} placeholder="Choose your review options, then generate a handoff prompt." />
        {generatedSignature && <p className="handoff-hint" role="status">{outdated ? "Options changed. Generate again to update the prompt." : "Edit the generated prompt if needed, then copy it to your agent."}</p>}
      </div>
      {error && <p className="project-error" role="alert">{error}</p>}
    </div>
  </Modal>;
}

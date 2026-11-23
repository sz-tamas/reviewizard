import { useState } from "react";
import { CopyButton } from "./CopyButton";
import { AGENT_PROMPTS, renderPrompt, type ReviewVars } from "./review/prompts";
import { Modal } from "./Modal";
import { ReviewStats, type ReviewStatsProps } from "./ReviewStats";

export function CompleteReviewModal({ repoPath, vars, stats, onComplete, onClose }: {
  repoPath: string;
  vars: ReviewVars;
  stats: ReviewStatsProps;
  onComplete: () => Promise<string | null>;
  onClose: () => void;
}) {
  const [pending, setPending] = useState(false);
  const [error, setError] = useState("");

  const template = AGENT_PROMPTS[0];
  const handoff = renderPrompt(template.template, { ...vars, repoPath, author: template.author });

  async function complete() {
    setPending(true);
    setError("");
    try {
      const failure = await onComplete();
      if (failure) { setError(failure); setPending(false); }
      else onClose();
    } catch (e) {
      setError((e as Error).message);
      setPending(false);
    }
  }

  return <Modal title="Complete Review" onClose={() => { if (!pending) onClose(); }} className="complete-review-modal">
    <div className="completion-options">
      <section className="completion-prompt-panel" aria-label="Address the review">
        <h3>Address the review</h3>
        <label className="handoff-format">
          Agent / format
          <select disabled>
            <option>Generic agent (Markdown)</option>
          </select>
        </label>
        <p className="prompt-hint">Works with any local agent that can access the repository and the reviewizard API.</p>
        <textarea className="prompt-editor" aria-label="Address the review prompt" value={handoff} readOnly />
        <div className="completion-card-action">
          <CopyButton text={handoff} idleLabel="Copy" />
        </div>
      </section>
      <section className="completion-manual-panel" aria-label="Close manually">
        <h3>Close manually</h3>
        <ReviewStats {...stats} compact />
        <p>Mark this review as completed. Findings and file-review marks are retained.</p>
        {error && <p className="project-error" role="alert">{error}</p>}
        <div className="completion-card-action">
          <button className="btn" onClick={() => void complete()} disabled={pending || stats.completed}>{pending ? "Completing…" : stats.completed ? "Completed" : "Complete review"}</button>
        </div>
      </section>
    </div>
  </Modal>;
}

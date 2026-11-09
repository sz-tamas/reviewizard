import { useState } from "react";
import { ReplyPromptEditor, type ReplyPromptEditorProps } from "./ReplyPromptEditor";
import { Modal } from "./Modal";
import { ReviewStats, type ReviewStatsProps } from "./ReviewStats";

export function CompleteReviewModal({ stats, onComplete, onClose, ...prompt }: ReplyPromptEditorProps & {
  stats: ReviewStatsProps;
  onComplete: () => Promise<string | null>;
  onClose: () => void;
}) {
  const [pending, setPending] = useState(false);
  const [error, setError] = useState("");

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
      <ReplyPromptEditor {...prompt} />
    </section>
    <section className="completion-manual-panel" aria-label="Close manually">
      <h3>Close manually</h3>
      <ReviewStats {...stats} />
      <p>Mark this review as completed. Findings and file-review marks are retained.</p>
      {error && <p className="project-error" role="alert">{error}</p>}
      <div className="project-dialog-actions">
        <button className="btn" onClick={onClose} disabled={pending}>Cancel</button>
        <button className="project-primary" onClick={() => void complete()} disabled={pending || stats.completed}>{pending ? "Completing…" : stats.completed ? "Completed" : "Complete review"}</button>
      </div>
    </section>
    </div>
  </Modal>;
}

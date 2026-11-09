import { useState } from "react";
import { useQuery } from "@tanstack/react-query";
import { ArrowRight, GitBranch } from "lucide-react";
import { api } from "../api/api";
import type { Project, Review, Side } from "../api/types";
import { Modal } from "./Modal";
import { ViewToggle } from "./ViewToggle";

export function ReviewSetupModal({ projects, initialProject, onClose, onSaved }: {
  projects: Project[];
  initialProject: Project;
  onClose: () => void;
  onSaved: (review: Review) => void;
}) {
  const [projectId, setProjectId] = useState(initialProject.id);
  const [mode, setMode] = useState<"changes" | "codebase">("changes");
  const [head, setHead] = useState("");
  const [base, setBase] = useState("");
  const [from, setFrom] = useState("all");
  const [side, setSide] = useState<Side>("head");
  const [pending, setPending] = useState(false);
  const [error, setError] = useState("");
  const project = projects.find((p) => p.id === projectId) ?? initialProject;
  const repo = `@project/${project.id}`;
  const branches = useQuery({
    queryKey: ["project", project.id, "branches"],
    queryFn: async () => (await api.branches(repo)).branches ?? [],
    enabled: mode === "changes",
  });
  const list = branches.data ?? [];
  const current = list.find((b) => b.isCurrent)?.name;
  const selectedHead = head || current || list.find((b) => !b.isRemote)?.name || "";
  const main = list.find((b) => b.isMain)?.name ?? "";
  const isCurrent = selectedHead === current;
  const sameBase = mode === "changes" && from === "all" && selectedHead === (base || main);
  const selectedSide: Side = !isCurrent ? "head" : sameBase && side === "head" ? "worktree" : side;
  const commits = useQuery({
    queryKey: ["review-setup", project.id, selectedHead, base, "commits"],
    queryFn: async () => (await api.commits(repo, selectedHead, base, 200)).commits ?? [],
    enabled: mode === "changes" && !!selectedHead,
  });

  async function save() {
    setPending(true);
    setError("");
    try {
      const review = await api.saveProjectReview(project.id, {
        mode, head: mode === "codebase" ? "HEAD" : selectedHead, base: mode === "codebase" ? "" : base, from: mode === "codebase" ? "all" : from, side: mode === "codebase" ? "worktree" : selectedSide,
      });
      onSaved(review);
    } catch (e) {
      setError((e as Error).message);
      setPending(false);
    }
  }

  return (
    <Modal title="Set up review" onClose={() => { if (!pending) onClose(); }} className="review-setup-modal">
      <div className="review-setup-body">
        <p>Choose the repository and scope to save on Projects.</p>
        <ViewToggle<"changes" | "codebase"> ariaLabel="Review scope" value={mode} onChange={setMode} disabled={pending}
          options={[{ value: "changes", label: "Changes" }, { value: "codebase", label: "Codebase" }]} />
        <div className={`review-setup-references${mode === "codebase" ? " review-setup-codebase" : ""}`}>
          <label>Repository
            <select aria-label="Review repository" value={project.id} disabled={pending} onChange={(e) => {
              setProjectId(Number(e.target.value)); setHead(""); setBase(""); setFrom("all"); setSide("head"); setError("");
            }}>
              {projects.map((p) => <option key={p.id} value={p.id}>{p.name} — {p.path}</option>)}
            </select>
          </label>
          {mode === "changes" && <>
          <span className="review-setup-separator">/</span>
          <label><span><GitBranch size={15} />Head branch</span>
            <select aria-label="Review head branch" value={selectedHead} disabled={pending || !list.length} onChange={(e) => { setHead(e.target.value); setFrom("all"); }}>
              {!list.length && <option value="">{branches.isPending ? "Loading…" : "No local branches"}</option>}
              {list.filter((b) => !b.isRemote).map((b) => <option key={b.name} value={b.name}>{b.name}</option>)}
            </select>
          </label>
          <ArrowRight className="review-setup-separator" size={20} />
          <label>Merge into / base
            <select aria-label="Review base branch" value={base} disabled={pending || !selectedHead} onChange={(e) => { setBase(e.target.value); setFrom("all"); }}>
              <option value="">auto{main ? ` (${main})` : ""}</option>
              {list.map((b) => <option key={b.name} value={b.name}>{b.name}{b.isRemote ? " (remote, last fetched)" : ""}</option>)}
            </select>
          </label>
          </>}
        </div>
        {mode === "changes" && <div className="review-setup-range">
          <label>from
            <select aria-label="Review starting commit" value={from} disabled={pending || !selectedHead || commits.isPending} onChange={(e) => setFrom(e.target.value)}>
              <option value="all">All (whole branch)</option>
              {(commits.data ?? []).map((c) => <option key={c.sha} value={c.sha}>{c.shortSha} — {c.subject}</option>)}
            </select>
          </label>
          <span>to</span>
          <ViewToggle<Side> ariaLabel="Review source side" value={selectedSide} onChange={setSide} disabled={pending || !selectedHead}
            options={[
              { value: "head", label: "Committed", disabled: sameBase && isCurrent, title: sameBase ? "This branch is its own base; select another base or a starting commit for a committed comparison" : undefined },
              { value: "index", label: "Staged", disabled: !isCurrent, title: "Includes committed changes and the Git index" },
              { value: "worktree", label: "Working tree", disabled: !isCurrent, title: "Includes committed changes, staged and unstaged edits, and untracked files" },
            ]} />
        </div>}
        <p className="review-setup-note">{mode === "codebase"
          ? "Review the entire codebase as it is in this folder, including unchanged files, local edits and non-ignored untracked files."
          : `${!isCurrent ? "Staged and working-tree comparisons require the checked-out branch. " : ""}A selected starting commit is included in the diff. Saving never switches branches or merges code.`}</p>
        {(error || (mode === "changes" && (branches.error || commits.error))) && <p role="alert" className="project-error">{error || (mode === "changes" ? branches.error?.message || commits.error?.message : "")}</p>}
        <div className="project-dialog-actions">
          <button className="btn" onClick={onClose} disabled={pending}>Cancel</button>
          <button className="project-primary" onClick={() => void save()} disabled={pending || (mode === "changes" && (!selectedHead || !!branches.error))}>{pending ? "Saving…" : "Save review"}</button>
        </div>
      </div>
    </Modal>
  );
}

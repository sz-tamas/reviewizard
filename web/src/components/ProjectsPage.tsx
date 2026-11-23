import { useState } from "react";
import { useQuery, useQueryClient } from "@tanstack/react-query";
import { AlertTriangle, ArrowUp, BookOpen, CheckCircle2, Clock3, Code2, FileText, Folder, GitBranch, Plus, Scale, Trash2 } from "lucide-react";
import { api } from "../api/api";
import type { Project, Review } from "../api/types";
import { repositoriesQueryOptions } from "../api/queries";
import { absoluteTime, relativeTime } from "../utils/time";
import { ReviewSetupModal } from "./ReviewSetupModal";
import { LanguageIcon } from "./FileTypeIcon";
import { AppHeader } from "./AppHeader";
import { ReviewIdentity, reviewSourceLabel } from "./ReviewIdentity";
import { ReviewStats } from "./ReviewStats";
import { Modal } from "./Modal";

const projectsKey = ["projects"];

function AddProject({ onClose, onAdded }: { onClose: () => void; onAdded: (project: Project) => void }) {
  const [folder, setFolder] = useState("");
  const [draft, setDraft] = useState<string>();
  const [pending, setPending] = useState(false);
  const [error, setError] = useState("");
  const browse = useQuery({ queryKey: ["project-folders", folder], queryFn: ({ signal }) => api.projectFolders(folder, signal), retry: false });
  const currentPath = browse.data?.path ?? folder;
  const windowsPath = /^(?:[a-z]:[\\/]|\\\\)/i.test(draft ?? currentPath);
  const input = draft ?? "";
  const normalized = windowsPath ? input.replace(/\\/g, "/") : input;
  const absolute = windowsPath || normalized.startsWith("/");
  const separator = absolute ? normalized.lastIndexOf("/") : -1;
  const directory = separator >= 0 ? normalized.slice(0, separator + 1) : "";
  const filter = normalized.slice(separator + 1).toLowerCase();
  const browsePath = windowsPath ? currentPath.replace(/\\/g, "/") : currentPath;
  const completeInCurrent = directory.replace(/\/$/, "") === browsePath.replace(/\/$/, "");
  const suggestions = useQuery({
    queryKey: ["project-folders", directory],
    queryFn: ({ signal }) => api.projectFolders(directory, signal),
    enabled: !!directory && !completeInCurrent,
    retry: false,
  });
  const listing = directory && !completeInCurrent ? suggestions : browse;
  const matches = listing.data?.folders.filter((entry) => entry.name.toLowerCase().includes(filter)) ?? [];
  const path = draft === undefined ? currentPath : input && !absolute && currentPath
    ? `${currentPath.replace(/[\\/]$/, "")}/${input}` : input;

  function navigate(next: string) {
    setFolder(next);
    setDraft(undefined);
    setError("");
  }

  async function add() {
    setPending(true);
    setError("");
    try { onAdded(await api.addProject(path)); }
    catch (e) { setError((e as Error).message); setPending(false); }
  }

  return (
    <Modal title="Add Project" onClose={onClose} className="project-folder-modal">
      <form className="project-folder-body" onSubmit={(e) => { e.preventDefault(); void add(); }}>
        <p>Choose a local Git repository containing <code>.git</code>. Files stay on this computer.</p>
        <label className="project-path-label">Repository folder
          <input autoFocus value={draft ?? currentPath} onChange={(e) => setDraft(e.target.value)} placeholder="/path/to/repository" disabled={pending} />
        </label>
        <div className="project-folder-roots">
          <button type="button" className="btn" onClick={() => navigate(path)} disabled={!path || pending}>Browse path</button>
          {(listing.data ?? browse.data)?.roots.map((root) => <button type="button" className="btn" key={root} onClick={() => navigate(root)} disabled={pending} title={root}>{root}</button>)}
        </div>
        {listing.isPending && <p role="status">Loading folders…</p>}
        {listing.error && <p className="project-error" role="alert">{listing.error.message}</p>}
        {listing.data && <div className="project-folder-browser">
          <div className="project-folder-current">
            <button type="button" className="btn btn-icon" aria-label="Parent folder" disabled={!listing.data.parent || pending} onClick={() => navigate(listing.data!.parent!)}><ArrowUp size={16} /></button>
            <span>{listing.data.path}</span>
            {listing.data.isRepository && <span className="project-git-label">Git repository</span>}
          </div>
          <div className="project-folder-list">
            {matches.map((entry) => <button type="button" key={entry.path} disabled={pending} onClick={() => navigate(entry.path)}>
              <Folder size={20} /><span>{entry.name}</span>{entry.isRepository && <span className="project-git-label">Git</span>}
            </button>)}
            {matches.length === 0 && <p>{filter ? "No matching folders." : "No subfolders. You can select this folder if it is a Git repository."}</p>}
          </div>
        </div>}
        {error && <p className="project-error" role="alert">{error}</p>}
        <div className="project-dialog-actions">
          <button type="button" className="btn" onClick={onClose} disabled={pending}>Cancel</button>
          <button type="submit" className="project-primary" disabled={!path || pending}>{pending ? "Adding…" : "Add Project"}</button>
        </div>
      </form>
    </Modal>
  );
}

function ProjectCard({ project, selected, onSelect, onRemove, onConfigure }: {
  project: Project; selected: boolean; onSelect: () => void; onRemove: () => void; onConfigure: () => void;
}) {
  const reviews = useQuery({ queryKey: ["project", project.id, "reviews"], queryFn: ({ signal }) => api.projectReviews(project.id, signal), refetchInterval: 15000 });
  return (
    <article className={`project-card${selected ? " is-selected" : ""}`}>
      <div className="project-card-identity">
        <Folder className="project-folder-icon" size={34} />
        <div className="project-card-description">
          <button className="project-name" onClick={onSelect} aria-pressed={selected}>{project.name}</button>
          <p className="project-path" title={project.path}>{project.path}</p>
        </div>
      </div>
      <div className="project-saved-actions">
        <div className="project-card-stats">
          <span><FileText />{project.reviewCount} reviews</span>
          <span><AlertTriangle className="project-warning" />{project.openFindings} open</span>
          <span><CheckCircle2 className="project-purple" />{project.resolvedFindings} resolved</span>
          <span title={project.lastActivity}><Clock3 />{relativeTime(project.lastActivity) || "Just added"}</span>
        </div>
        <button className="project-remove" aria-label={`Remove project ${project.name}`} title="Remove project" onClick={onRemove}>
          <Trash2 size={18} aria-hidden="true" />
        </button>
      </div>
      <ProjectMetadata metadata={project.metadata} />
      <div className="project-saved-reviews" aria-label={`Saved reviews for ${project.name}`}>
        {reviews.isPending && <p role="status">Loading saved reviews…</p>}
        {reviews.error && <p className="project-error" role="alert">{reviews.error.message}</p>}
        {reviews.data?.map((review) => <SavedReviewCard key={review.id} review={review} repoName={project.name} />)}
        <button type="button" className="project-add-review-card" onClick={onConfigure}>
          <Plus size={24} aria-hidden="true" />
          Add review
        </button>
      </div>
    </article>
  );
}

function ProjectMetadata({ metadata }: { metadata: Project["metadata"] }) {
  if (!metadata) return <p className="project-metadata">Repository metadata unavailable</p>;

  return (
    <ul className="project-metadata" aria-label="Repository metadata">
      <li><Scale aria-hidden="true" /><span>{metadata.license ? `License: ${metadata.license}` : "No license found"}</span></li>
      <li><BookOpen aria-hidden="true" /><span>{metadata.readme ?? "No README found"}</span></li>
      <li className="project-metadata-branch"><GitBranch aria-hidden="true" /><span>{metadata.branch ?? "Detached HEAD"}</span></li>
      {metadata.languages.length ? metadata.languages.map(({ name, version }) => (
        <li key={name} data-language={name}><LanguageIcon name={name} /><span>{name}{version && ` ${version}`}</span></li>
      )) : <li><Code2 aria-hidden="true" /><span>No languages detected in root</span></li>}
    </ul>
  );
}

function SavedReviewCard({ review, repoName }: { review: Review; repoName: string }) {
  const client = useQueryClient();
  const [removing, setRemoving] = useState(false);
  const [pending, setPending] = useState(false);
  const [error, setError] = useState("");
  const stats = useQuery({
    queryKey: ["project-review-stats", review.id, review.updatedAt],
    queryFn: async () => {
      const [current, diff] = await Promise.all([api.getReview(review.id), api.reviewDiff(review.id)]);
      if (current.annotationError) throw new Error(current.annotationError);
      const files = diff.files ?? [];
      const reviewed = new Set(current.reviewedFiles ?? []);
      const findings = (current.comments ?? []).filter((comment) => !comment.resolved);
      return {
        fileCount: files.length,
        reviewedCount: files.filter((file) => reviewed.has(file.newPath || file.oldPath)).length,
        openCommentCount: findings.length,
        findingCounts: {
          bugs: findings.filter((comment) => comment.type === "bug").length,
          suggestions: findings.filter((comment) => comment.type === "suggestion").length,
          other: findings.filter((comment) => comment.type !== "bug" && comment.type !== "suggestion").length,
        },
        completed: current.status === "completed",
      };
    },
    refetchInterval: 15000,
  });
  async function remove() {
    setPending(true); setError("");
    try {
      await api.removeReview(review.id);
      await Promise.all([
        client.invalidateQueries({ queryKey: ["project"] }),
        client.invalidateQueries({ queryKey: projectsKey }),
      ]);
    } catch (e) { setError((e as Error).message); }
    finally { setPending(false); }
  }
  return <section className={`project-review-entry${review.status === "completed" ? " is-completed" : ""}`} aria-label={`Review ${review.id}${review.status === "completed" ? " (completed)" : ""}`}>
    <div className="project-review-header">
      <a className="project-review-link" aria-label="Open Review" href={`/reviews?review=${review.id}`}>
        <ReviewIdentity heading="h2" repoName={repoName} head={review.headRef} base={review.baseRef}
          from={review.scopeFrom || "all"} source={reviewSourceLabel(review.scopeSide ?? "head")}
          review={review} fileCount={stats.data?.fileCount} />
      </a>
      <button className="project-remove" aria-label={`Remove review ${review.id}`} title="Remove review" onClick={() => { setError(""); setRemoving(true); }}>
        <Trash2 size={18} aria-hidden="true" />
      </button>
    </div>
    <div className="project-review-dates">
      <span>Created <time dateTime={review.createdAt} title={absoluteTime(review.createdAt)}>{relativeTime(review.createdAt) || "—"}</time></span>
      <span>Last edited <time dateTime={review.updatedAt} title={absoluteTime(review.updatedAt)}>{relativeTime(review.updatedAt) || "—"}</time></span>
    </div>
    {stats.isPending && <p role="status">Loading review statistics…</p>}
    {stats.error && <p className="project-error" role="alert">Unable to load review statistics: {stats.error.message}</p>}
    {stats.data && <ReviewStats {...stats.data} compact completed={review.status === "completed"} />}
    {removing && <Modal title="Remove review?" className="modal-sm" close="none" onClose={() => { if (!pending) setRemoving(false); }}>
      <div className="confirm-body">
        <p>Remove <strong>{review.headRef}</strong> and its findings? This can't be undone.</p>
        {error && <p role="alert" className="project-error">{error}</p>}
      </div>
      <div className="confirm-actions">
        <button className="btn" data-autofocus disabled={pending} onClick={() => setRemoving(false)}>Cancel</button>
        <button className="btn danger" disabled={pending} onClick={() => void remove()}>{pending ? "Removing…" : "Remove"}</button>
      </div>
    </Modal>}

  </section>;
}

export function ProjectsPage() {
  const client = useQueryClient();
  const projects = useQuery({ queryKey: projectsKey, queryFn: async ({ signal }) => (await api.projects(signal)) ?? [], refetchInterval: 15000 });
  const [adding, setAdding] = useState(false);
  const [configuring, setConfiguring] = useState<Project>();
  const [selected, setSelected] = useState<number>();
  const [removing, setRemoving] = useState<Project>();
  const [pending, setPending] = useState(false);
  const [error, setError] = useState("");
  const all = projects.data ?? [];
  const active = all.find((p) => p.id === selected) ?? all[0];
  async function refresh() {
    await Promise.all([client.invalidateQueries({ queryKey: projectsKey }), client.invalidateQueries({ queryKey: repositoriesQueryOptions().queryKey })]);
  }
  async function remove() {
    if (!removing) return;
    setPending(true); setError("");
    try { await api.removeProject(removing.id); setRemoving(undefined); await refresh(); }
    catch (e) { setError((e as Error).message); }
    finally { setPending(false); }
  }
  return (
    <div className="app projects-app">
      <AppHeader />
      <main className="projects-page">
        <section className="projects-heading">
          <div>
            <h1><Folder size={32} />Projects</h1>
            <p>Manage your local repositories and run code reviews on any branch.</p>
          </div>
          <button className="project-secondary" onClick={() => setAdding(true)}><Plus size={20} />Add Project</button>
        </section>
        <div className="projects-content">
          {projects.isPending && <p role="status" className="projects-empty">Loading projects…</p>}
          {projects.error && <div className="projects-empty" role="alert"><p>{projects.error.message}</p><button className="btn" onClick={() => void projects.refetch()}>Retry</button></div>}
          {!projects.isPending && !projects.error && !all.length && <div className="projects-empty"><Folder size={40} /><h2>Add your first project</h2><p>Choose a local folder containing .git to start reviewing.</p></div>}
          <div className="project-cards">{all.map((project) => <ProjectCard key={project.id} project={project} selected={active?.id === project.id} onSelect={() => setSelected(project.id)} onRemove={() => { setError(""); setRemoving(project); }} onConfigure={() => setConfiguring(project)} />)}</div>
        </div>
      </main>
      {adding && <AddProject onClose={() => setAdding(false)} onAdded={(p) => { setSelected(p.id); setAdding(false); void refresh(); }} />}
      {configuring && <ReviewSetupModal initialProject={configuring} projects={all.some((p) => p.id === configuring.id) ? all : [...all, configuring]} onClose={() => setConfiguring(undefined)} onSaved={() => { setConfiguring(undefined); void refresh(); void client.invalidateQueries({ queryKey: ["project"] }); }} />}
      {removing && <Modal title="Remove project?" className="modal-sm" close="none" onClose={() => { if (!pending) setRemoving(undefined); }}>
        <div className="confirm-body">
          <p>Remove <strong>{removing.name}</strong> from Projects? The folder and all existing reviews and findings will be kept. Adding the same folder restores this project.</p>
          {error && <p role="alert" className="project-error">{error}</p>}
        </div>
        <div className="confirm-actions">
          <button className="btn" data-autofocus disabled={pending} onClick={() => setRemoving(undefined)}>Cancel</button>
          <button className="btn danger" disabled={pending} onClick={() => void remove()}>{pending ? "Removing…" : "Remove"}</button>
        </div>
      </Modal>}
    </div>
  );
}

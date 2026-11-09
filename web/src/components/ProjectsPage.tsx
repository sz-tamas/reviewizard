import { useEffect, useState } from "react";
import { useQuery, useQueryClient } from "@tanstack/react-query";
import { AlertTriangle, ArrowUp, CheckCircle2, Clock3, FileText, Folder, GitBranch, MoreHorizontal, Play, Plus, Search } from "lucide-react";
import { api } from "../api/api";
import type { Project, Review } from "../api/types";
import { repositoriesQueryOptions } from "../api/queries";
import { relativeTime } from "../utils/time";
import { ReviewSetupModal } from "./ReviewSetupModal";
import { AppHeader } from "./AppHeader";
import { Modal } from "./Modal";

const projectsKey = ["projects"];

function AddProject({ onClose, onAdded }: { onClose: () => void; onAdded: (project: Project) => void }) {
  const [folder, setFolder] = useState("");
  const [path, setPath] = useState("");
  const [pending, setPending] = useState(false);
  const [error, setError] = useState("");
  const listing = useQuery({ queryKey: ["project-folders", folder], queryFn: ({ signal }) => api.projectFolders(folder, signal), retry: false });
  useEffect(() => { if (listing.data) setPath(listing.data.path); }, [listing.data]);

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
          <input autoFocus value={path} onChange={(e) => setPath(e.target.value)} placeholder="/path/to/repository" disabled={pending} />
        </label>
        <div className="project-folder-roots">
          <button type="button" className="btn" onClick={() => setFolder(path)} disabled={!path || pending}>Browse path</button>
          {listing.data?.roots.map((root) => <button type="button" className="btn" key={root} onClick={() => setFolder(root)} disabled={pending} title={root}>{root}</button>)}
        </div>
        {listing.isPending && <p role="status">Loading folders…</p>}
        {listing.error && <p className="project-error" role="alert">{listing.error.message}</p>}
        {listing.data && <div className="project-folder-browser">
          <div className="project-folder-current">
            <button type="button" className="btn btn-icon" aria-label="Parent folder" disabled={!listing.data.parent || pending} onClick={() => setFolder(listing.data!.parent!)}><ArrowUp size={16} /></button>
            <span>{listing.data.path}</span>
            {listing.data.isRepository && <span className="project-git-label">Git repository</span>}
          </div>
          <div className="project-folder-list">
            {listing.data.folders.map((entry) => <button type="button" key={entry.path} disabled={pending} onClick={() => setFolder(entry.path)}>
              <Folder size={20} /><span>{entry.name}</span>{entry.isRepository && <span className="project-git-label">Git</span>}
            </button>)}
            {listing.data.folders.length === 0 && <p>No subfolders. You can select this folder if it is a Git repository.</p>}
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
  const [menu, setMenu] = useState(false);
  const reviews = useQuery({ queryKey: ["project", project.id, "reviews"], queryFn: ({ signal }) => api.projectReviews(project.id, signal), refetchInterval: 15000 });
  return (
    <article className={`project-card${selected ? " is-selected" : ""}`}>
      <div className="project-card-identity">
        <Folder className="project-folder-icon" size={34} />
        <div className="project-card-description">
          <button className="project-name" onClick={onSelect} aria-pressed={selected}>{project.name}</button>
          <p className="project-path" title={project.path}>{project.path}</p>
          <div className="project-card-stats">
            <span><FileText />{project.reviewCount} reviews</span>
            <span><AlertTriangle className="project-warning" />{project.openFindings} open</span>
            <span><CheckCircle2 className="project-purple" />{project.resolvedFindings} resolved</span>
            <span title={project.lastActivity}><Clock3 />{relativeTime(project.lastActivity) || "Just added"}</span>
          </div>
        </div>
      </div>
      <div className="project-saved-actions">
        <button className="project-primary" onClick={onConfigure}><Plus size={18} />Add review</button>
        <div className="project-overflow">
          <button className="project-more" aria-label={`More actions for ${project.name}`} aria-expanded={menu} onClick={() => setMenu(!menu)}><MoreHorizontal size={22} /></button>
          {menu && <div className="project-action-menu"><button onClick={() => { setMenu(false); onRemove(); }}>Remove project</button></div>}
        </div>
      </div>
      <div className="project-saved-reviews" aria-label={`Saved reviews for ${project.name}`}>
        {reviews.isPending && <p role="status">Loading saved reviews…</p>}
        {reviews.error && <p className="project-error" role="alert">{reviews.error.message}</p>}
        {reviews.data?.length === 0 && <p>No saved reviews yet. Choose Changes or Codebase and a source side with Add review.</p>}
        {reviews.data?.map((review) => <SavedReviewRow key={review.id} review={review} />)}
      </div>
    </article>
  );
}

function SavedReviewRow({ review }: { review: Review }) {
  const source = review.scopeSide === "index" ? "Staged" : review.scopeSide === "worktree" ? "Working tree" : "Committed";
  return <div className="project-saved-review">
    <GitBranch size={18} />
    <div><strong>{review.headRef}{review.scopeMode === "codebase" ? " · Entire codebase" : ` → ${review.baseRef}`}</strong><span>{source}{review.scopeConfigured === false ? " (legacy default)" : ""}{review.scopeMode !== "codebase" && <> • from {review.scopeFrom && review.scopeFrom !== "all" ? `${review.scopeFrom.slice(0,7)} (inclusive)` : "All (whole branch)"}</>} • {review.status === "completed" ? "Completed" : "In progress"}</span></div>
    <a className="project-secondary" href={`/reviews?review=${review.id}`}><Play size={16} />Open Review</a>
  </div>;
}

export function ProjectsPage() {
  const client = useQueryClient();
  const projects = useQuery({ queryKey: projectsKey, queryFn: async ({ signal }) => (await api.projects(signal)) ?? [], refetchInterval: 15000 });
  const [search, setSearch] = useState("");
  const [adding, setAdding] = useState(false);
  const [configuring, setConfiguring] = useState<Project>();
  const [selected, setSelected] = useState<number>();
  const [removing, setRemoving] = useState<Project>();
  const [pending, setPending] = useState(false);
  const [error, setError] = useState("");
  const all = projects.data ?? [];
  const visible = all.filter((p) => `${p.name} ${p.path}`.toLowerCase().includes(search.toLowerCase()));
  const active = visible.find((p) => p.id === selected) ?? visible[0];
  const totals = all.reduce((s,p) => ({ reviews: s.reviews + p.reviewCount, open: s.open + p.openFindings, resolved: s.resolved + p.resolvedFindings }), { reviews: 0, open: 0, resolved: 0 });
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
          <div><h1><Folder size={32} />Projects</h1><p>Manage your local repositories and run code reviews on any branch.</p></div>
          <div className="projects-totals" aria-label="Project statistics">
            {[{ icon: Folder, count: all.length, label: "Projects", color: "project-purple" }, { icon: FileText, count: totals.reviews, label: "Total reviews", color: "" }, { icon: AlertTriangle, count: totals.open, label: "Open findings", color: "project-warning" }, { icon: CheckCircle2, count: totals.resolved, label: "Resolved", color: "project-green" }].map(({ icon: Icon, count, label, color }) => <div className="project-total" key={label}><Icon className={color} size={26} /><div><strong>{projects.isPending ? "—" : count}</strong><span>{label}</span></div></div>)}
          </div>
        </section>
        <div className="projects-content">
          <div className="projects-tools">
            <label className="project-search"><Search size={20} /><input aria-label="Search projects" placeholder="Search projects…" value={search} onChange={(e) => setSearch(e.target.value)} /></label>
            <button className="project-secondary" onClick={() => setAdding(true)}><Plus size={20} />Add Project</button>
          </div>
          {projects.isPending && <p role="status" className="projects-empty">Loading projects…</p>}
          {projects.error && <div className="projects-empty" role="alert"><p>{projects.error.message}</p><button className="btn" onClick={() => void projects.refetch()}>Retry</button></div>}
          {!projects.isPending && !projects.error && !visible.length && <div className="projects-empty"><Folder size={40} /><h2>{all.length ? "No matching projects" : "Add your first project"}</h2><p>{all.length ? "Try another name or folder path." : "Choose a local folder containing .git to start reviewing."}</p>{!all.length && <button className="project-primary" onClick={() => setAdding(true)}><Plus size={18} />Add Project</button>}</div>}
          <div className="project-cards">{visible.map((project) => <ProjectCard key={project.id} project={project} selected={active?.id === project.id} onSelect={() => setSelected(project.id)} onRemove={() => setRemoving(project)} onConfigure={() => setConfiguring(project)} />)}</div>
        </div>
      </main>
      {adding && <AddProject onClose={() => setAdding(false)} onAdded={(p) => { setSelected(p.id); setAdding(false); void refresh(); }} />}
      {configuring && <ReviewSetupModal initialProject={configuring} projects={all.some((p) => p.id === configuring.id) ? all : [...all, configuring]} onClose={() => setConfiguring(undefined)} onSaved={() => { setConfiguring(undefined); void refresh(); void client.invalidateQueries({ queryKey: ["project"] }); }} />}
      {removing && <Modal title="Remove project" onClose={() => { if (!pending) setRemoving(undefined); }}><div className="project-folder-body"><p>Remove <strong>{removing.name}</strong> from Projects? The folder and all existing reviews and findings will be kept. Adding the same folder restores this project.</p>{error && <p role="alert" className="project-error">{error}</p>}<div className="project-dialog-actions"><button className="btn" disabled={pending} onClick={() => setRemoving(undefined)}>Cancel</button><button className="btn danger" disabled={pending} onClick={() => void remove()}>{pending ? "Removing…" : "Remove project"}</button></div></div></Modal>}
    </div>
  );
}

import type { ComboOption } from "./Combobox";
import { AppHeader } from "./AppHeader";
import { ReviewStats } from "./ReviewStats";
import { IconTrash } from "./icons";
import { Check, ChevronDown, Code2, Download, RefreshCw, Sparkles } from "lucide-react";
import { DropdownMenu } from "radix-ui";
import type { DiffStat } from "../components/diff/diffStats";
import type { Review, Side } from "../api/types";

// Repo/head/base pickers + the diff-view controls + reload.
export interface Selection {
  repo: string;
  repoOptions: ComboOption[];
  onRepoChange: (v: string) => void;
  head: string;
  headOptions: ComboOption[];
  onHeadChange: (v: string) => void;
  base: string;
  baseOptions: ComboOption[];
  onBaseChange: (v: string) => void;
  baseRelevant: boolean;
  from: string;
  fromOptions: ComboOption[];
  onFromChange: (v: string) => void;
  commitCount: number;
  headIsCurrent: boolean;
  // The base resolves to head, so the committed range is empty by construction.
  baseIsHead: boolean;
  side: Side;
  onSideChange: (v: Side) => void;
  loading: boolean;
  onReload: () => void;
}

export interface TopBarActions {
  onShowPrompts: () => void;
  onShowExport: () => void;
  onReset: () => void;
  onComplete?: () => void;
}

// Review status shown on the right (only when a review is open).
export interface TopBarStatus {
  review: Review | null;
  shortSha?: string;
  baseSha: string;
  // Files the diff itself changes, never the synthetic cards: this is the number compared with a git client.
  fileCount: number;
  stat: DiffStat;
  openCommentCount: number;
  canReset: boolean;
  reviewedCount?: number;
  findingCounts?: { bugs: number; suggestions: number; other: number };
}

interface Props {
  selection: Selection;
  actions: TopBarActions;
  status: TopBarStatus;
}

export function TopBar({ selection: s, actions, status }: Props) {
  const repoName = s.repoOptions.find((r) => r.value === s.repo)?.label ?? s.repo;
  const source = s.side === "head" ? "Committed" : s.side === "index" ? "Staged" : "Working tree";
  const completed = status.review?.status === "completed";
  const codebase = status.review?.scopeMode === "codebase";
  const scopeTitle = codebase ? `Entire codebase: ${source} on ${s.head}` : `Before: ${status.baseSha || status.review?.baseRef || s.base || "auto"} — ${s.from === "all" ? "merge-base with the selected base" : `parent of ${s.from}, so that commit is included`}\nAfter: ${source}${s.side === "worktree" ? " (committed, staged, unstaged and untracked)" : s.side === "index" ? " (committed plus the Git index)" : ` on ${s.head}`}`;
  return (
    <>
      <AppHeader />
      <section className="review-overview" aria-label="Review overview">
        <div className="review-identity">
          <div className="review-emblem"><Code2 size={27} /></div>
          <div>
            <h1>{repoName || "Review"}<span> / </span>{s.head || "Select a saved review"}</h1>
            <p title={scopeTitle}>Review: {source}{status.review?.scopeConfigured === false ? " (legacy default)" : ""}{!codebase && <><span> • </span>Base: {status.review?.baseRef || s.base || "auto"}</>}<span> • </span>Scope: {codebase ? "Entire codebase" : "Changed files"} ({status.fileCount})</p>
            {!codebase && s.from !== "all" && <p className="review-from">From {s.from.slice(0, 7)} (inclusive)</p>}
          </div>
        </div>
        <ReviewStats {...status} completed={completed} />
        <div className="review-header-actions">
          <button className="review-action-primary" onClick={actions.onShowPrompts} disabled={!status.review}><Sparkles size={17} />AI Review Handoff</button>
          <div className="review-complete-group">
            <button className="review-action" onClick={actions.onComplete} disabled={!status.review || !actions.onComplete}><Check size={17} />{completed ? "Reopen Review" : "Complete Review"}</button>
            <DropdownMenu.Root>
              <DropdownMenu.Trigger asChild>
                <button className="review-action review-menu-trigger" disabled={!status.review} aria-label="More review actions"><ChevronDown size={16} /></button>
              </DropdownMenu.Trigger>
              <DropdownMenu.Portal>
                <DropdownMenu.Content className="review-action-menu" align="end" sideOffset={6}>
                  <DropdownMenu.Item onSelect={s.onReload} disabled={s.loading || !s.head}><RefreshCw size={16} />Reload</DropdownMenu.Item>
                  <DropdownMenu.Item onSelect={actions.onShowExport} disabled={!status.review}><Download size={16} />Export review</DropdownMenu.Item>
                  <DropdownMenu.Separator />
                  <DropdownMenu.Item className="review-menu-danger" onSelect={actions.onReset} disabled={!status.canReset}><IconTrash />Reset review</DropdownMenu.Item>
                </DropdownMenu.Content>
              </DropdownMenu.Portal>
            </DropdownMenu.Root>
          </div>
        </div>
      </section>
    </>
  );
}

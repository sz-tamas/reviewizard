import { useEffect, useMemo, useRef, useState } from "react";
import { AddFileModal } from "./components/AddFileModal";
import { AgentPromptsModal } from "./components/AgentPromptsModal";
import { CompleteReviewModal } from "./components/CompleteReviewModal";
import { CommentRefPopover } from "./components/CommentRefPopover";
import { CommentsPanel } from "./components/CommentsPanel";
import { DiffView } from "./components/DiffView";
import { ExportModal } from "./components/ExportModal";
import { FileExplorer, orderedFiles } from "./components/FileExplorer";
import { FindBar } from "./components/FindBar";
import { LazyFile } from "./components/LazyFile";
import { PaneRail } from "./components/PaneRail";
import { ResetConfirmModal } from "./components/ResetConfirmModal";
import { ReviewSummary } from "./components/ReviewSummary";
import { TopBar } from "./components/TopBar";
import { UndoToast } from "./components/UndoToast";
import { EmptyState } from "./components/EmptyState";
import {
  IconFileDiff,
  IconFolder,
  IconGitBranch,
  IconGitCommit,
  IconX,
} from "./components/icons";
import { useActiveFile } from "./hooks/useActiveFile";
import { useCommentActions } from "./hooks/useCommentActions";
import { useCommentRefs } from "./hooks/useCommentRefs";
import { useJump } from "./hooks/useJump";
import { useOccurrenceHighlight } from "./hooks/useOccurrenceHighlight";
import { usePanelResize } from "./hooks/usePanelResize";
import { useReview, type InitialReviewSelection } from "./hooks/useReview";
import { useUndoableDelete } from "./hooks/useUndoableDelete";
import { useUnseenActivity } from "./hooks/useUnseenActivity";
import { pruneDrafts, showDraftsFor, useLeaveWarning } from "./components/comments/drafts";
import { setFontsRepo } from "./preferences/fonts/fonts";
import { setLangMapRepo } from "./preferences/langMap";
import { setThemeRepo } from "./preferences/theme";
import type { CommentFilter } from "./components/comments/commentFilter";
import { NO_FILTER, authorsOf, filterComments } from "./components/comments/commentFilter";
import type { CommentSort } from "./components/comments/commentSort";
import { isCommentSort, sortComments } from "./components/comments/commentSort";
import { awaitingYouCount } from "./components/comments/commentTurn";
import { commentsFor, groupByPath } from "./components/comments/commentsByPath";
import { commentSignal, pathSignal } from "./components/comments/cardSignals";
import { CommentIdsContext } from "./components/comments/commentRef";
import { diffRowHeight, estFileHeight } from "./components/diff/fileHeight";
import { useDefaultLayout } from "./preferences/diffLayout";
import { offsetOf, useFonts } from "./preferences/fonts/fonts";
import { totalStat } from "./components/diff/diffStats";
import type { FileDiff } from "./api/types";
import { effectivePath } from "./api/types";
import { LS, getString, setString, writeBasePref } from "./preferences/storage";

export default function App({ initialSelection }: { initialSelection?: InitialReviewSelection } = {}) {
  const {
    repos,
    reposLoaded,
    repo,
    changeRepo,
    branches,
    branchesLoaded,
    head,
    changeHead,
    base,
    setBase,
    review,
    files,
    baseSha,
    comments: storedComments,
    setComments,
    reviewedFiles,
    from,
    setFrom,
    commitCount,
    headIsCurrent,
    baseIsHead,
    loading,
    error,
    setError,
    side,
    changeSide,
    shortSha,
    repoOptions,
    headOptions,
    baseOptions,
    fromOptions,
    startReview,
    resetReview,
    completeReview,
    setReviewedPaths,
    toggleReviewed,
    setSummary,
  } = useReview(initialSelection);

  const pendingDelete = useUndoableDelete({ setComments, setError, reviewId: review?.id });
  const { hidden: hiddenComments } = pendingDelete;
  const comments = useMemo(
    () =>
      hiddenComments.size === 0
        ? storedComments
        : storedComments.filter((c) => !hiddenComments.has(c.id)),
    [storedComments, hiddenComments]
  );

  const reviewStats = {
    fileCount: files.length,
    reviewedCount: files.filter((f) => reviewedFiles.has(f.newPath || f.oldPath)).length,
    openCommentCount: comments.filter((c) => !c.resolved).length,
    findingCounts: {
      bugs: comments.filter((c) => !c.resolved && c.type === "bug").length,
      suggestions: comments.filter((c) => !c.resolved && c.type === "suggestion").length,
      other: comments.filter((c) => !c.resolved && c.type !== "bug" && c.type !== "suggestion").length,
    },
  };

  // Files opened only to comment on. Session state: the comment-bearing ones re-derive from `comments` on reload.
  const [openedFiles, setOpenedFiles] = useState<string[]>([]);
  const [showAddFile, setShowAddFile] = useState(false);
  const [showExport, setShowExport] = useState(false);
  const [showPrompts, setShowPrompts] = useState(false);
  const [showCompletion, setShowCompletion] = useState(false);
  const [confirmingReset, setConfirmingReset] = useState(false);
  const [commentSort, setCommentSort] = useState<CommentSort>(() => {
    const stored = getString(LS.commentSort);
    return isCommentSort(stored) ? stored : "file";
  });
  const [commentFilter, setCommentFilter] = useState<CommentFilter>(NO_FILTER);
  const diffColRef = useRef<HTMLDivElement>(null);
  const explorerSearchRef = useRef<HTMLInputElement>(null);
  // Plain state on purpose: the React Compiler keeps unchanged DiffViews from re-rendering on it.
  const [selectedFile, setSelectedFile] = useState<string | null>(null);

  const {
    leftW,
    rightW,
    leftOpen,
    rightOpen,
    toggleLeft,
    toggleRight,
    gridTemplateColumns,
    mainRef,
    startResize,
    onResizeKey,
  } = usePanelResize();
  const fonts = useFonts();
  const { suppress: suppressActiveFile } = useActiveFile(diffColRef, setSelectedFile, review?.id);
  const {
    activeComment,
    expandTarget,
    expandComment,
    jumpTo,
    jumpToFile,
    jumpToDraft,
    resetJump,
  } = useJump({
    comments,
    setSelectedFile,
    rootRef: diffColRef,
    onProgrammaticScroll: suppressActiveFile,
  });
  const [showFullSignal, setShowFullSignal] = useState<{ path: string; n: number } | null>(null);
  const highlight = useOccurrenceHighlight(!!review, diffColRef);
  const refHover = useCommentRefs(jumpTo);
  const { commentActions, handleAddComment } = useCommentActions({
    review,
    setComments,
    setError,
    side,
    onDelete: pendingDelete.remove,
  });


  const unseen = useUnseenActivity(comments, review?.id);
  useLeaveWarning();

  useEffect(() => {
    const badge = unseen > 0 ? `(${unseen}) ` : "";
    document.title = review
      ? `${badge}${repo} · ${review.headRef} → ${review.baseRef} — reviewizard`
      : "reviewizard";
  }, [review, repo, unseen]);

  const reviewId = review ? review.id : null;
  // A filter carried into another review would silently hide its comments.
  useEffect(() => {
    setCommentFilter(NO_FILTER);
    showDraftsFor(reviewId);
  }, [reviewId]);

  // After showDraftsFor: pruning checks the review it is handed is the one whose drafts are shown.
  useEffect(() => {
    if (review) pruneDrafts(review.id, review.comments ?? []);
  }, [review]);

  // Both stores ignore the empty repo the first render carries, having seeded from the remembered one.
  useEffect(() => {
    setThemeRepo(repo);
    setFontsRepo(repo);
    setLangMapRepo(repo);
  }, [repo]);

  // Keyed on repo alone, deliberately (resetJump isn't a dep); useReview resets its own data.
  useEffect(() => {
    if (!repo) return;
    setSelectedFile(null);
    setOpenedFiles([]);
    resetJump();
  }, [repo]);

  function showFullFile() {
    const path = highlight.path;
    if (path) setShowFullSignal((s) => ({ path, n: (s?.n ?? 0) + 1 }));
  }

  // One predicate behind both canReset and requestReset's guard, so they can't drift apart.
  const hasReviewState = comments.length > 0 || reviewedFiles.size > 0 || !!review?.summary;

  function requestReset() {
    if (!review || !hasReviewState) return;
    setConfirmingReset(true);
  }

  function performReset() {
    setConfirmingReset(false);
    resetReview();
  }

  function openFile(path: string) {
    setShowAddFile(false);
    setOpenedFiles((s) => (s.includes(path) ? s : [...s, path]));
    // The card mounts on the next render; jumpToFile's aim waits for it.
    jumpToFile(path);
  }

  // Synthetic cards for paths the diff didn't touch: opened here, or anchoring a comment — which
  // is also what restores them after a reload.
  const allFiles = useMemo(() => {
    const inDiff = new Set(files.map((f) => f.newPath || f.oldPath));
    const extras = new Set<string>();
    for (const p of openedFiles) if (p && !inDiff.has(p)) extras.add(p);
    for (const c of comments) {
      const p = effectivePath(c);
      if (p && !inDiff.has(p)) extras.add(p);
    }
    const synthetic: FileDiff[] = [...extras].map((p) => ({
      oldPath: p,
      newPath: p,
      status: "unchanged",
      hunks: [],
    }));
    return [...files, ...synthetic];
  }, [files, openedFiles, comments]);

  // Off the real diff: the synthetic cards in `allFiles` have nothing to count.
  const diffStat = useMemo(() => totalStat(files), [files]);

  const orderedDiffFiles = useMemo(() => orderedFiles(allFiles), [allFiles]);
  const orderedFilePaths = useMemo(
    () => orderedDiffFiles.map((f) => f.newPath || f.oldPath),
    [orderedDiffFiles]
  );
  // Keyed on the joined id list, not the array: a no-op SSE refetch must keep the Set's
  // identity, or every <Markdown> re-runs markdown-it + Shiki.
  const commentIdKey = comments.map((c) => c.id).join(",");
  const commentIds = useMemo(
    () => new Set(commentIdKey ? commentIdKey.split(",").map(Number) : []),
    [commentIdKey]
  );
  // Per-file slices are what let DiffView's memo skip cards whose comments didn't change.
  const commentsByPath = useMemo(() => groupByPath(comments), [comments]);

  // Off the live font size, not a constant: the placeholder has to be drawn at the scale the card
  // will mount at, or every card below it jumps when it does.
  const rowH = diffRowHeight(offsetOf(fonts, "monoOffset"));
  const layout = useDefaultLayout();
  const estHeight = (f: FileDiff) => {
    const path = f.newPath || f.oldPath;
    return estFileHeight({
      file: f,
      reviewed: reviewedFiles.has(path),
      comments: commentsFor(commentsByPath, path),
      rowH,
      layout,
    });
  };

  // Apply the selected ordering and filter to the comments pane.
  const sortedComments = useMemo(
    () => sortComments(filterComments(comments, commentFilter), commentSort, orderedFilePaths),
    [comments, commentFilter, commentSort, orderedFilePaths]
  );
  const commentAuthors = useMemo(() => authorsOf(comments), [comments]);
  // Off the unfiltered list on purpose: this count describes the review, not the pane.
  const awaitingYou = useMemo(() => awaitingYouCount(comments), [comments]);

  return (
    <div className="app review-page">
      <TopBar
        selection={{
          repo,
          repoOptions,
          onRepoChange: (v) => {
            changeRepo(v);
            setString(LS.repo, v);
          },
          head,
          headOptions,
          onHeadChange: changeHead,
          base,
          baseOptions,
          onBaseChange: (v) => {
            setBase(v);
            writeBasePref(repo, v);
          },
          baseRelevant: from === "all",
          from,
          fromOptions,
          onFromChange: setFrom,
          commitCount,
          headIsCurrent,
          baseIsHead,
          side,
          onSideChange: changeSide,
          loading,
          onReload: startReview,
        }}
        actions={{
          onShowPrompts: () => setShowPrompts(true),
          onShowExport: () => setShowExport(true),
          onReset: requestReset,
          onComplete: () => { if (review?.status === "completed") void completeReview(); else setShowCompletion(true); },
        }}
        status={{
          review,
          shortSha,
          baseSha,
          ...reviewStats,
          stat: diffStat,
          canReset: hasReviewState,
        }}
      />

      {error && (
        <div className="error banner" role="alert">
          <span>{error}</span>
          <button
            className="banner-dismiss"
            onClick={() => setError(null)}
            title="Dismiss"
            aria-label="Dismiss error"
          >
            <IconX />
          </button>
        </div>
      )}

      {/* Not an error: nothing failed, the server just couldn't read the repo to judge staleness. */}
      {review?.annotationError && (
        <div className="warn banner" role="status">
          <span>
            {review.annotationError}. Comment staleness and reviewed marks aren’t being
            checked — what you see is the last saved state.
          </span>
        </div>
      )}

      {!review &&
        !error &&
        (!reposLoaded ? (
          <div className="empty">
            <span className="spinner" aria-hidden="true" />
            Loading…
          </div>
        ) : repos.length === 0 ? (
          <EmptyState
            icon={<IconFolder />}
            title="No git repositories found"
            hint={<>Add a local Git repository on the <a href="/projects">Projects page</a> to start a review.</>}
          />
        ) : branchesLoaded && branches.length === 0 ? (
          <EmptyState
            icon={<IconGitCommit />}
            title={`${repo} has no commits yet`}
            hint="There is nothing to review until the repository has at least one commit."
          />
        ) : (
          <EmptyState
            icon={<IconGitBranch />}
            title="Select a branch to start a review"
            hint="Create a saved review on the Projects page to choose its repository, head, base, starting commit and source side."
          />
        ))}

      {review && (
        <div className="main" ref={mainRef} style={{ gridTemplateColumns }}>
          <aside className={`explorer-column${leftOpen ? "" : " pane-collapsed"}`}>
            {!leftOpen && (
              <PaneRail label="Files" count={allFiles.length} side="left" onExpand={toggleLeft} />
            )}
            {leftOpen && (
              <FileExplorer
                scopeSummary={`${review.repoPath}\n${review.headRef} → ${review.baseRef}\nFrom: ${from === "all" ? "All (whole branch)" : `${from} (inclusive)`}\nSource: ${side === "head" ? "Committed" : side === "index" ? "Staged" : "Working tree"}`}
                files={allFiles}
                comments={comments}
                reviewed={reviewedFiles}
                selected={selectedFile}
                onSelect={jumpToFile}
                onToggleReviewed={toggleReviewed}
                onToggleFolder={setReviewedPaths}
                onAddFile={() => setShowAddFile(true)}
                onCollapse={toggleLeft}
                searchRef={explorerSearchRef}
              />
            )}
          </aside>
          {/* Inert while shut: a drag would clamp the stored width back up to the minimum. */}
          <div
            className={`resizer${leftOpen ? "" : " resizer-inert"}`}
            role="separator"
            aria-orientation="vertical"
            aria-label="Resize files panel"
            aria-valuemin={160}
            aria-valuemax={560}
            aria-valuenow={leftW}
            tabIndex={leftOpen ? 0 : -1}
            onMouseDown={leftOpen ? (e) => startResize(e, "left") : undefined}
            onKeyDown={leftOpen ? (e) => onResizeKey(e, "left") : undefined}
          />
          <div className="diff-pane">
            <div className="diff-column" ref={diffColRef}>
              {allFiles.length === 0 && loading && (
                <div className="empty">
                  <span className="spinner" aria-hidden="true" />
                  Loading diff…
                </div>
              )}
              {allFiles.length === 0 && !loading && (
                <EmptyState
                  icon={<IconFileDiff />}
                  title={review?.scopeMode === "codebase" ? "No files to review" : "No changes to review"}
                  hint={review?.scopeMode === "codebase" ? "The selected codebase source contains no files." : "Nothing differs in this saved comparison. Create another review on Projects to choose a different base, starting commit or source side."}
                />
              )}
              <CommentIdsContext.Provider value={commentIds}>
                {orderedDiffFiles.map((f) => {
                  const path = f.newPath || f.oldPath;
                  const fileComments = commentsFor(commentsByPath, path);
                  return (
                    <LazyFile
                      key={path}
                      anchorId={`file-${path}`}
                      label={path}
                      estHeight={estHeight(f)}
                      rootRef={diffColRef}
                    >
                      <DiffView
                        file={f}
                        repo={repo}
                        headRef={review.headRef}
                        baseRef={baseSha}
                        side={side}
                        comments={fileComments}
                        onAddComment={handleAddComment}
                        actions={commentActions}
                        reviewed={reviewedFiles.has(path)}
                        onToggleReviewed={toggleReviewed}
                        expandTarget={pathSignal(expandTarget, path)}
                        expandComment={commentSignal(expandComment, fileComments)}
                        showFullSignal={pathSignal(showFullSignal, path)}
                        activeComment={commentSignal(activeComment, fileComments)}
                      />
                    </LazyFile>
                  );
                })}
              </CommentIdsContext.Provider>
            </div>
            {highlight.term && (
              <FindBar
                term={highlight.term}
                count={highlight.count}
                index={highlight.index}
                changedOnly={highlight.viewMode === "changed"}
                onNext={highlight.next}
                onPrev={highlight.prev}
                onShowFullFile={showFullFile}
                onClear={highlight.clear}
              />
            )}
          </div>
          <div
            className={`resizer${rightOpen ? "" : " resizer-inert"}`}
            role="separator"
            aria-orientation="vertical"
            aria-label="Resize comments panel"
            aria-valuemin={220}
            aria-valuemax={640}
            aria-valuenow={rightW}
            tabIndex={rightOpen ? 0 : -1}
            onMouseDown={rightOpen ? (e) => startResize(e, "right") : undefined}
            onKeyDown={rightOpen ? (e) => onResizeKey(e, "right") : undefined}
          />
          <aside className={`side-column${rightOpen ? "" : " pane-collapsed"}`}>
            {!rightOpen && (
              <PaneRail
                label="Comments"
                count={comments.length}
                side="right"
                onExpand={toggleRight}
              />
            )}
            {rightOpen && (
              <>
                <ReviewSummary
                  summary={review.summary}
                  onSave={setSummary}
                  onCollapse={toggleRight}
                />
                <CommentsPanel
                  comments={sortedComments}
                  total={comments.length}
                  awaitingYou={awaitingYou}
                  sort={commentSort}
                  onSortChange={(v) => {
                    setCommentSort(v);
                    setString(LS.commentSort, v);
                  }}
                  filter={commentFilter}
                  onFilterChange={setCommentFilter}
                  authors={commentAuthors}
                  onJump={jumpTo}
                  onJumpToDraft={jumpToDraft}
                  fileOrder={orderedFilePaths}
                  onDelete={commentActions.onDelete}
                />
              </>
            )}
          </aside>
        </div>
      )}

      {showAddFile && review && (
        <AddFileModal
          repo={repo}
          headRef={review.headRef}
          present={new Set(orderedFilePaths)}
          onSelect={openFile}
          onClose={() => setShowAddFile(false)}
        />
      )}

      {showExport && review && (
        <ExportModal reviewId={review.id} onClose={() => setShowExport(false)} />
      )}

      {showPrompts && review && (
        // Keyed on repo so a switch remounts the editor instead of carrying drafts across.
        <AgentPromptsModal
          repoPath={review.repoPath}
          key={repo}
          repo={repo}
          vars={{
            origin: window.location.origin,
            reviewId: review.id,
            headRef: review.headRef,
            baseRef: review.baseRef,
            side,
            fromRef: from,
            mode: review.scopeMode,
          }}
          onClose={() => setShowPrompts(false)}
        />
      )}

      {showCompletion && review && <CompleteReviewModal
        key={review.id}
        repoPath={review.repoPath}
        vars={{ origin: window.location.origin, reviewId: review.id, headRef: review.headRef, baseRef: review.baseRef, side, fromRef: from, mode: review.scopeMode }}
        stats={{ ...reviewStats, completed: review.status === "completed" }}
        onComplete={completeReview}
        onClose={() => setShowCompletion(false)}
      />}

      {confirmingReset && (
        <ResetConfirmModal
          commentCount={comments.length}
          reviewedCount={reviewedFiles.size}
          hasSummary={!!review?.summary}
          onCancel={() => setConfirmingReset(false)}
          onConfirm={performReset}
        />
      )}

      <CommentRefPopover hovered={refHover} comments={comments} />

      {pendingDelete.pending !== null && (
        <UndoToast
          key={pendingDelete.pending}
          message={`Deleted comment #${pendingDelete.pending}`}
          onUndo={pendingDelete.undo}
        />
      )}
    </div>
  );
}

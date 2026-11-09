export interface ReviewStatsProps {
  fileCount: number;
  reviewedCount?: number;
  openCommentCount: number;
  findingCounts?: { bugs: number; suggestions: number; other: number };
  completed: boolean;
}

export function ReviewStats({ fileCount, reviewedCount = 0, openCommentCount, findingCounts, completed }: ReviewStatsProps) {
  const percent = fileCount ? Math.round(reviewedCount / fileCount * 100) : 0;
  return <div className="review-metrics">
    <div className="review-metric">
      <span>Files reviewed</span>
      <strong>{reviewedCount} / {fileCount} ({percent}%)</strong>
      <div className="review-progress" role="progressbar" aria-label="Files reviewed" aria-valuemin={0} aria-valuemax={fileCount || 1} aria-valuenow={reviewedCount}><i style={{ width: `${percent}%` }} /></div>
    </div>
    <div className="review-metric">
      <span>Findings</span>
      <div className="review-findings"><strong>{openCommentCount}</strong>{findingCounts && <div className="review-finding-badges"><span className="finding-bugs" title="Bugs">{findingCounts.bugs}</span><span className="finding-suggestions" title="Suggestions">{findingCounts.suggestions}</span><span className="finding-other" title="Other findings">{findingCounts.other}</span></div>}</div>
    </div>
    <div className="review-metric review-state"><span>Status</span><strong><i className={completed ? "completed" : ""} />{completed ? "Completed" : "In progress"}</strong></div>
  </div>;
}

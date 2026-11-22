import { Code2 } from "lucide-react";
import type { Review, Side } from "../api/types";

interface Props {
  repoName: string;
  head: string;
  base: string;
  from: string;
  source: string;
  review?: Review | null;
  fileCount?: number;
  scopeTitle?: string;
  heading?: "h1" | "h2";
}

export function reviewSourceLabel(side: Side): string {
  return side === "head" ? "Committed" : side === "index" ? "Staged" : "Working tree";
}

export function ReviewIdentity({ repoName, head, base, from, source, review, fileCount, scopeTitle, heading: Heading = "h1" }: Props) {
  const codebase = review?.scopeMode === "codebase";
  return (
    <div className="review-identity">
      <div className="review-emblem"><Code2 size={27} aria-hidden="true" /></div>
      <div>
        <Heading>{repoName || "Review"}<span> / </span>{head || "Select a saved review"}</Heading>
        <p title={scopeTitle}>
          Review: {source}{review?.scopeConfigured === false ? " (legacy default)" : ""}
          {!codebase && <><span> • </span>Base: {review?.baseRef || base || "auto"}</>}
          <span> • </span>Scope: {codebase ? "Entire codebase" : "Changed files"}{fileCount !== undefined && ` (${fileCount})`}
        </p>
        {!codebase && from !== "all" && <p className="review-from">From {from.slice(0, 7)} (inclusive)</p>}
      </div>
    </div>
  );
}

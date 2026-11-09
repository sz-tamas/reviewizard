import { createRoute } from "@tanstack/react-router";
import { useQuery } from "@tanstack/react-query";
import { rootRoute } from "./__root";
import App from "../App";
import { api } from "../api/api";
import { AppHeader } from "../components/AppHeader";

export const reviewsRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: "/reviews",
  validateSearch: (search: Record<string, unknown>): { review?: number } => {
    const id = Number(search.review);
    return { review: Number.isSafeInteger(id) && id > 0 ? id : undefined };
  },
  component: Reviews,
});
function Reviews() {
  const { review: id } = reviewsRoute.useSearch();
  return id ? <SavedReview key={id} id={id} /> : <ReviewNotice />;
}
function SavedReview({ id }: { id: number }) {
  const saved = useQuery({ queryKey: ["review", id, "setup"], queryFn: () => api.getReview(id) });
  const repositories = useQuery({ queryKey: ["repositories"], queryFn: async ({ signal }) => (await api.repos(signal)).repos ?? [] });
  if (saved.error || repositories.error) return <ReviewNotice error={(saved.error ?? repositories.error)!.message} />;
  if (!saved.data || !repositories.data) return <ReviewNotice loading />;
  const review = saved.data;
  const repo = repositories.data.find((r) => r.path === review.repoPath);
  if (!repo) return <ReviewNotice error="Register this review's repository on Projects before opening it." />;
  return <App initialSelection={{ project: repo.name, branch: review.headRef, base: review.baseRef, from: review.scopeFrom ?? "all", side: review.scopeSide ?? "head", reviewId: review.id, repoPath: review.repoPath }} />;
}
function ReviewNotice({ loading, error }: { loading?: boolean; error?: string }) {
  return <div className="app review-page">
    <AppHeader />
    <main className="empty">
      <h1>{loading ? "Loading review…" : error ? "Review unavailable" : "Choose a saved review"}</h1>
      {error && <p role="alert">{error}</p>}
      {!loading && <a className="project-primary" href="/projects">Open Projects</a>}
    </main>
  </div>;
}

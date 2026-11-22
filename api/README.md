# Backend

Rust serves only the API. The frontend is served separately by Vite. Build and run through the root mise tasks:

```sh
REVIEWIZARD_ROOT=/path/to/repository mise run start
mise run serve -- --root /path/to/repository
mise run check
```

`api/main.rs` collects configuration and startup wiring. `api/routes/` contains
HTTP handlers and route registration. `api/services/` owns Git, SQLite, reviews,
Markdown export, middleware and live updates. `api/utils/` holds
shared logic independent of HTTP. Cargo manifests and lockfile live at the root.

## API

Local projects: GET/POST `/api/projects`; DELETE `/api/projects/{id}` removes a
project from the list without deleting its folder or review history. POST accepts
`{"path":"/absolute/local/git/root"}`. Folders must contain `.git` and validate as
Git worktree roots; linked worktrees are supported. Re-adding a path restores its
stable ID. A path that resolves to a different canonical folder is rejected.
GET includes a `metadata` summary for each project. It checks root-level README
and license files, reads the current Git branch, and infers languages and declared
toolchain versions from root filenames and common manifests. The scan does not
recurse, run project tools, or write to the repository. File reads are bounded
and skip symlinks. A missing version is returned as null, and files with an
unrecognized license are reported by filename.
GET `/api/project-folders?path=...` lists directories only, confined to the home
and startup folders; hidden folders and symlinks are excluded. An absolute path
outside those browse roots can be registered explicitly.

Repository/Git endpoints accept the stable `@project/{id}` handle as `repo`.
`/api/repos` includes active registered projects plus startup-discovered repositories
for compatibility, deduplicated by logical path. Registration never changes the
startup root. Project removal preserves old review access and stored anchors.
Project statistics count persisted reviews and comment resolution per exact path.

GET `/api/projects/{id}/reviews` lists the project's saved and legacy reviews.
POST to the same URL saves a new independent assignment with
`{"mode":"changes","head":"feature","base":"main","from":"all","side":"head"}`. `from` may be a
commit/ref (normalized to a commit SHA, inclusive); side is `head`, `index` or
`worktree`. An empty base selects the default branch, which is resolved and stored.
The optional `mode` defaults to `changes`, including for existing reviews.
Use `{"mode":"codebase","head":"main","side":"head"}` to review the entire
codebase, including unchanged files. Codebase mode requires an empty/omitted base
and `from: "all"` (or omitted); it needs no default branch or merge target.
The setup UI uses `head: "HEAD"` and `side: "worktree"` automatically for
Codebase mode, without branch discovery or branch/source controls. These folder
reviews also work with detached HEAD and repositories with no commits. The API also supports
explicit codebase source scopes for existing assignments. All three source sides are supported. Working tree includes non-ignored untracked
files; committed and index sources include their tracked files. Ignored untracked
files and submodule contents are outside the repository inventory.
GET `/api/reviews/{id}/diff` derives exactly the saved scope. Codebase files are
represented as additions against an empty tree, preserving source line numbers
and binary metadata. The UI and generated AI handoff identify the entire-codebase
scope; handoffs request inspection of unchanged files as well. Missing refs,
non-ancestor starting commits and incompatible index/worktree checkouts are errors,
without scope fallback. Comments and reviewed-file writes for configured reviews
must use the recorded source side. Legacy APIs keep their existing resume behavior
and never reuse a configured review's ID.

POST `/api/reviews/{id}/source` accepts `{"side":"head"}`, `index`, or `worktree`.
It validates and saves the chosen source before notifying viewers. Existing finding
anchors retain their source; reviewed-file progress uses marks for the selected source.

POST `/api/reviews/{id}/status` accepts `{"status":"completed"}` or `draft`.
Completion is independent of reviewed-file freshness; exporting does not reopen it.
Additive migrations retain existing IDs/paths/anchors and label legacy scope defaults.
Explicit saved assignments are excluded from automatic legacy draft pruning.

Repository/Git reads: `/api/repos`, `/api/branches`, `/api/commits`, `/api/diff`,
`/api/files`, `/api/file`, `/api/blob`. Select committed, index or working-tree
content explicitly. `/healthz` identifies the backend; UI paths such as `/` return JSON 404 responses.

DELETE `/api/reviews/{id}` permanently removes a review and its findings, replies
and reviewed-file marks; missing reviews return 404. The Projects page keeps completed
reviews selectable with a completed status, and shows review progress.

Review lifecycle: POST `/api/reviews`; GET `/api/reviews/{id}`; POST its `/reset`,
`/summary`, `/reviewed`, `/export` and `/export.md` operations. GET `/events` streams
coalesced metadata, diff and ref notifications.

Comments: GET/POST `/api/reviews/{id}/comments`; PATCH/DELETE `/api/comments/{id}`;
POST its `/resolved` and `/replies` operations; PATCH/DELETE `/api/replies/{id}`.
POST `/api/comments/{id}/resolved` with `{"resolved":true}` resolves a finding;
`{"resolved":false}` reopens it. Success returns 204; a missing comment returns 404.
Replies and findings are retained, and resolving findings does not complete the review.
Exports with `instructions=true` ask agents to fix and verify findings, post an
explanatory reply, then resolve verified fixes (including already-fixed issues).
Disputed findings, clarification questions and unverified fixes stay open.

Mutations validate input before writing and notify after success. JSON bodies are
bounded to 8 MiB and must contain one document. Query keys use their first value.
Services can run directly without HTTP. Error responses contain a JSON `error`.

## Storage and review state

SQLite uses WAL, foreign keys, one serialized connection and transactional additive
migrations. Reviews contain comments/replies, summaries and reviewed-file marks.
Existing database files remain supported. The first open in a selected directory
creates an integrity-checked snapshot before changes; existing snapshots are retained.

Snippets and reviewed hashes are source-side specific. Read-time annotation follows
unchanged lines through moves/renames and marks edited or missing anchors outdated.
Immutable diff caching never caches a review verdict or a failed Git command.
Markdown generation and optional agent reply instructions live in `services/export/`.

## Verification

`services/tests/integration.rs` and service unit tests exercise real Git fixtures,
SQLite schema/mutations, relocation, caching, HTTP boundaries and SSE. Native
standalone acceptance under `services/tests/acceptance/` tests persisted review
state, database snapshots/restores, API-only routing, streaming and shutdown using
disposable repositories/data directories. It needs Python 3 and currently runs on POSIX.

See [running and data recovery](../docs/RUNNING.md) for commands and backup behavior.
Feature expansion remains tracked separately in [PLAN-REFACT.md](../PLAN-REFACT.md).

---
name: reviewizard
description: Develop and verify this reviewizard project's Rust API and React frontend, including review scope, agent handoff, persistence, live updates and frontend-led multi-session work. Use for changes to this application, not generic reviews of unrelated repositories.
---

# Develop reviewizard

Read [AGENTS.md](AGENTS.md) for the current architecture, commands and invariants.
Use [api/README.md](api/README.md) for backend/service details and
[docs/RUNNING.md](docs/RUNNING.md) for runtime/data recovery. Do not load all sources
or start servers unless the requested change needs them.

## Establish the real implementation boundary

Confirm the requested behavior and inspect the relevant implementation and tests.
Check `mise.toml` and `process-compose.yaml` for runtime work; check
`web/package.json` and `web/vite.config.ts` for frontend/tooling work. They take
precedence over historical assumptions about embedded assets or startup scripts.

The API and UI run separately. Projects registers local Git roots in SQLite;
Reviews opens a saved assignment by ID, including its head/base, inclusive starting
commit and source side. The backend's saved-review diff endpoint owns that scope.
Stable project handles resolve
independently of the unchanged startup discovery root. Separate session management
is still planned. Preserve stored logical paths and canonical confinement.
Node.js and pnpm are pinned in mise; use pnpm and keep `web/pnpm-lock.yaml` aligned.

## Trace the change across its owners

| Requested change | Inspect and keep consistent |
| --- | --- |
| API/domain behavior | `api/routes/`, the relevant `api/services/` module, models and integration tests. Shared non-HTTP helpers belong in `api/utils/`. |
| UI/review interaction | The relevant `web/src/` component/hook, `types.ts`, API client and colocated tests. |
| Scope or agent handoff | Viewer selection, actual diff/content source, prompt variables/templates, stored source side, snippets/anchors and export instructions. |
| Persistence/freshness | Store models/schema/migrations, mutations, review annotation/hashes, cache identity and live notifications. |
| Discovery/multiple sessions | Existing workspace confinement, frontend selection, persisted repository identity, session ownership, cache/subscription isolation and lifecycle cleanup. |
| Development processes | mise tasks, process-compose commands/readiness/env, Vite proxy, CI and affected documentation. |

Keep HTTP decoding/formatting in routes; domain behavior in services; reusable logic
in utils. Keep Markdown generation on the backend. For multi-session changes, trace
which repository each request, finding, cache entry and SSE stream belongs to before
altering global state. Preserve existing reviews during that transition.

For agent assignments, explicitly compare the generated commands with the viewer's
saved scope. Generated handoffs combine selected lenses and severity into one
assignment and use the saved diff endpoint and source side. Edited output may differ
from its source settings; retired single-lens overrides remain stored but inactive.
Do not equate no findings with completed coverage, or
claim new review modes merely because unchanged files can be displayed.

## Implement and verify the requested behavior

Reuse existing patterns and keep unrelated product/tooling migrations separate.
Add meaningful regressions where behavior changes: source-side fidelity, persistence,
error classification, rejected mutations, notification ordering or session isolation.
Use existing Git/database fixtures rather than the user's active repositories/data.

Use `mise run check:rust` for Rust format/lint/tests and `mise run check:web` for
frontend tests. Use `mise run build:web` for TypeScript/build validation. For API
lifecycle/storage/streaming changes, run `mise run check:default` on POSIX or the
relevant native checks; `mise run check` combines the project's configured checks.
Inspect the current task definitions before relying on a remembered command.

When browser validation is needed, run separate API/UI processes with temporary
data, explicit nondefault ports and pruning disabled. Confirm that the frontend
proxy reaches that API. Exercise the changed action and verify persisted state or
SSE behavior, not just that a page loaded. Stop only the processes created for the
check and retain useful evidence under ignored build output.

Update instruction/documentation claims when behavior or commands change. Report
what the user can now do, what was tested and any remaining limitation. Do not
publish, deploy or introduce an embedded AI runner as an implied follow-up.

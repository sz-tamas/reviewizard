# Claude project entrypoint

@AGENTS.md

`AGENTS.md` is the shared source of project instructions, architecture boundaries,
commands and behavioral invariants. Read it before changing this repository.
For the implementation workflow, use [SKILL.md](SKILL.md) when developing the app.

The current development entrypoint is `mise run start`, supervised by
`process-compose.yaml`: Vite serves the UI on 5173 and proxies API calls to Rust on
7777. Rust is API-only. Do not restore the old embedded/single-binary frontend setup.

Frontend directory discovery and one UI with multiple review sessions are intended
next steps, not current capabilities. The backend still initializes one workspace
at startup. The frontend uses pnpm with the Node.js and pnpm versions pinned in mise.

For a narrower task, inspect only the relevant routes/services/hooks/tests. Follow
the current user request rather than treating the roadmap as authorization to
implement extra features. Keep these notes brief; maintain shared details in
`AGENTS.md` instead of duplicating them here.

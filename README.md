# reviewizard

A local code-review workspace for you and your coding agent. Browse Git diffs and
repository files, leave anchored findings and replies, track reviewed files, and
hand off review work through prompts, Markdown exports, or the HTTP API.

Register local repositories on the Projects page and save reviews with their own
comparison scope. Review state is stored locally in SQLite. Coding agents run
externally and interact with reviewizard through its handoff tools and API.

## Getting started

Install [mise](https://mise.jdx.dev/getting-started.html) and Git, then run from
the project directory:

```sh
mise trust
mise run install
mise run start
```

Open [http://127.0.0.1:5173](http://127.0.0.1:5173), register a local Git repository
on the Projects page, and create a review.

To use a different startup workspace:

```sh
REVIEWIZARD_ROOT=/path/to/repository mise run start
```

The React/Vite frontend and Rust API run as separate processes; the API listens on
port `7777`. For development with backend and frontend live reload, use
`mise run dev`. Run `mise run test` for frontend and Rust tests.

See [AGENTS.md](AGENTS.md) for project guidance and [api/README.md](api/README.md)
for API details.

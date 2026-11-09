# Frontend source layout

- `App.tsx`, `main.tsx`, `styles.css`: application composition, entry point and global styles.
- `app/`: TanStack Router setup and Query client factory.
- `routes/`: the code-based TanStack route tree; `/` mounts the existing review workspace.
- `api/`: HTTP client, shared API models and TanStack Query options.
- `components/`: React UI components and their tests; `ui/` holds shadcn primitives.
- `hooks/`: React hooks for review state and UI interactions.
- `comments/`: comment filtering, ordering, references, activity signals and drafts.
- `diff/`: diff rows, gaps, statistics, word matching and layout calculations.
- `review/`: review navigation and agent handoff prompts.
- `preferences/`: persisted settings, language mapping, diff layout, themes and font handling.
- `rendering/`: syntax highlighting and Mermaid rendering integrations.
- `utils/`: shared platform, time, scrolling and numeric helpers.
- `assets/fonts/`: bundled font files and their licenses.

Keep tests beside the module or component they exercise. Import modules directly;
folder barrel exports are unnecessary. Keep API models in `api/types.ts` and use
`App.tsx` to compose the UI rather than collecting domain logic there.

## Frontend stack

React 18 + Vite, TanStack Router v1, TanStack Query v5, Tailwind CSS v4 and
shadcn/ui (Radix, New York). Inter is bundled for the UI; JetBrains Mono is bundled
for code in every theme. Saved custom fonts and installed-font selection remain supported.

Use the Node.js and pnpm versions pinned in `mise.toml` and `web/package.json`.
From the project root:

```sh
mise run check:web
mise run build:web
mise exec -- pnpm --dir web run lint
mise exec -- pnpm dlx shadcn@latest add dialog --cwd web
```

`web/pnpm-lock.yaml` is the only frontend lockfile. Installation tasks disable
lifecycle scripts. The frontend still runs separately from Rust; Vite proxies `/api`,
including SSE, to `REVIEWIZARD_API_URL`.

`@/` resolves to `src/` in TypeScript, Vite and Vitest. Use the shared query client
provided in `main.tsx`; route loaders can access it via their context. Repository
listing uses Query; review mutations, diffs and coalesced SSE refreshes remain in
`useReview` to preserve source-side and refresh semantics. There is no dynamic
workspace/session selector yet. Future query keys must include repository/session
identity and all relevant scope/source axes.

Tailwind's theme and utilities are enabled without its global Preflight reset to
preserve existing Markdown, controls and diff geometry. Existing CSS lives in the
`components` cascade layer, so utilities can override it. New shadcn primitives
receive scoped control resets and semantic colors mapped to the existing review
theme tokens. The theme store also updates `.dark`, keeping dark variants aligned
with the resolved editor theme. Prefer `components/ui` primitives for new UI;
existing review components have not been comprehensively restyled.

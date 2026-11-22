# reviewizard

A local code-review workspace for you and your coding agent. Review changes or an
entire codebase in your browser, discuss findings next to the code, and keep track
of what you have reviewed across multiple repositories.

Everything you do in reviewizard is available through the UI. Your repositories
stay on your computer, and projects, reviews, findings and replies are saved locally.

## Get started

Install [mise](https://mise.jdx.dev/getting-started.html) and Git. From the project
folder, run the one-time setup:

```sh
mise trust
mise install
```

Then start the app:

```sh
mise start
```

Your default browser opens automatically when the app is ready. The UI is available
at [http://127.0.0.1:5173](http://127.0.0.1:5173) if you need to open it again.
The first launch can take longer while dependencies are installed and the app builds.
The API and UI run quietly in the background. Once ready, the terminal shows the
app URL and **Press Ctrl+C to stop**. Leave that terminal open while you review;
Ctrl+C stops both processes.

For development with live reload and the process-compose interface, use `mise dev`
instead. It also opens the browser.

## Add a project and save a review

1. On **Projects**, choose **Add Project** and browse to a local Git repository,
   or enter its folder path.
2. Choose **Add review** on the project card.
3. Pick a review scope and save it. Click the saved review row to open it.

Each project can have multiple independent reviews. Project cards show each review's
file coverage, findings and status, so you can return to unfinished work or revisit
a completed review.

### Review changes

Choose **Changes** to compare a head branch with a base branch. Review the whole
branch or choose a starting commit; that commit's changes are included.

Choose the version of the code you want to inspect:

| Source | What you see |
| --- | --- |
| **Committed** | Committed changes in the saved comparison. |
| **Staged** | The comparison including the Git index. |
| **Working tree** | The comparison including staged and unstaged edits, plus non-ignored untracked files. |

Staged and working-tree comparisons require the reviewed branch to be checked out.
Creating a review never switches branches or merges code.

The source dropdown beside **AI Review Handoff** shows the saved choice. Changing
it updates the saved review and reloads its content.

### Review a codebase

Choose **Codebase** to inspect the entire repository folder, including unchanged
files, local edits and non-ignored untracked files. This starts with the working tree
and does not require a comparison base or starting commit.

## Read the code and record findings

Use the file explorer to navigate the review and filter files. Inspect diffs in
split or unified layouts, expand context or view a full file, and search the code.
Markdown and supported media can be previewed alongside source views.

Add comments to a line or range of lines, choose a finding type, and discuss it
through replies. The comments panel brings threads together, with filters and
sorting to help you find open issues and conversations needing attention. You can
also add a repository file to comment on even if it is outside the changed files.

Findings retain their file, line range, captured text and original code source.
reviewizard checks their anchors against the current content and indicates when an
anchor has moved or become outdated. Committing code does not delete its comments;
source selection still matters when checking an existing anchor.

Mark files as **Reviewed**, individually or by folder. Coverage is shown in the
review header and on the project card. Reviewed marks are checked against file
content, so subsequent edits can make a file need review again.

The review updates as local files change and findings or replies arrive. You can
also reload it from the review actions menu.

## Work with a coding agent

Choose **AI Review Handoff** to prepare an assignment for your external coding agent:

- Combine review lenses: correctness, security, design, performance,
  maintainability and tests.
- Set the minimum severity to report.
- Choose an agent or handoff format, generate the prompt, and edit it if needed.
- Copy it to your agent to review the saved scope and report findings back.

reviewizard organizes the review; it does not run an autonomous agent. You choose
where to run the agent and which repository content to share with it. Returned
findings and replies appear in the UI.

## Complete, reopen or share a review

Choose **Complete Review** to either copy a prompt asking your agent to address the
review or mark it completed manually. Completion keeps the findings and reviewed-file
marks. Completed reviews remain visible on Projects with a green check and
**Completed** status. Open one and choose **Reopen Review** to resume it.

From the review actions menu, choose **Export review** to preview, copy or download
the review as Markdown. You can include instructions for an agent to reply to the
findings.

To delete a saved review, use its three-dot menu on Projects and confirm **Remove**.
This permanently deletes that review and its findings, replies and reviewed-file
marks. Removing a project from the list instead keeps its folder and review history;
adding the same folder restores the project.

## Local storage

Review data is stored in `~/.reviewizard/reviewizard.db`. Keep this folder if you want
to preserve your review history, and include it in your backups. Display preferences
are stored in your browser, so they can differ between browsers or browser profiles.

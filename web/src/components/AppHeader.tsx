import IconArrowRight from "@tabler/icons-react/icons/IconArrowRight";
import IconFileSearch from "@tabler/icons-react/icons/IconFileSearch";
import IconFolderPlus from "@tabler/icons-react/icons/IconFolderPlus";
import IconHelpCircle from "@tabler/icons-react/icons/IconHelpCircle";
import IconShieldLock from "@tabler/icons-react/icons/IconShieldLock";
import IconTools from "@tabler/icons-react/icons/IconTools";
import { Moon, Sun } from "lucide-react";
import { useState } from "react";
import { Brand } from "./Brand";
import { Modal } from "./Modal";
import { setAppearance, themeOf, useTheme } from "../preferences/theme";

function HandoffGuide() {
  const [expanded, setExpanded] = useState(false);

  return (
    <>
      <button
        type="button"
        className="btn app-help-guide-toggle"
        aria-expanded={expanded}
        onClick={() => setExpanded((open) => !open)}
      >
        {expanded ? "Read less" : "Read more"}
      </button>
      {expanded && (
        <div className="app-help-guide">
          <p>Keep reviewizard running and use a local AI tool that can access the repository and call the reviewizard API from the same machine. Select the same repository in a desktop app or start a CLI session from its folder. If your tool prompts for API access, check that the address matches the one in the handoff before approving.</p>
          <h4>Ask for a review</h4>
          <ol>
            <li>Open the saved review and select <strong>AI Review Handoff</strong>. Pick your review options and choose a format under <strong>Agent / format</strong>. The default, <strong>Generic agent (Markdown)</strong>, is a paste-ready prompt for local tools.</li>
            <li>Select <strong>Copy</strong> at the bottom of the dialog. Paste and submit it in your tool with permission to read the code and post findings to reviewizard.</li>
            <li>Return to the review to read the findings.</li>
          </ol>
          <h4>Address findings and complete the review</h4>
          <ol>
            <li>In the same review, select <strong>Complete Review</strong>. Use <strong>Copy</strong> in <strong>Address the review</strong> and paste the prompt into your tool.</li>
            <li>Give the tool read and write access to the repository and access to the local API. It can make and verify fixes, reply to findings, and resolve verified fixes.</li>
            <li>Check the changes and replies. When you’re ready to close the review, select <strong>Complete review</strong> under <strong>Close manually</strong>.</li>
          </ol>
        </div>
      )}
    </>
  );
}

export function AppHeader() {
  const dark = themeOf(useTheme()).mermaid === "dark";
  const [helpOpen, setHelpOpen] = useState(false);

  return (
    <header className="topbar app-header">
      <a className="header-brand" href="/" aria-label="reviewizard home">
        <Brand />
      </a>
      <div className="header-utilities">
        <span className="header-local"><span aria-hidden="true" className="header-local-dot" />Local (localhost)</span>
        <button
          type="button"
          className="header-icon"
          aria-label="Help"
          aria-haspopup="dialog"
          title="About reviewizard"
          onClick={() => setHelpOpen(true)}
        >
          <IconHelpCircle aria-hidden="true" />
        </button>
        <button
          type="button"
          className="header-icon"
          aria-label="Light theme"
          aria-pressed={!dark}
          title={dark ? "Switch to light theme" : "Switch to dark theme"}
          onClick={() => setAppearance(dark ? "github-light" : "github-dark")}
        >
          {dark ? <Sun aria-hidden="true" /> : <Moon aria-hidden="true" />}
        </button>
      </div>
      {helpOpen && (
        <Modal title="About reviewizard" className="modal-sm" close="autofocus" onClose={() => setHelpOpen(false)}>
          <div className="app-help">
            <p>reviewizard is a local workspace for reviewing Git changes and leaving comments on files and lines.</p>
            <section className="app-help-section app-help-privacy">
              <h3><IconShieldLock aria-hidden="true" />Local storage and privacy</h3>
              <p>Registered repositories are read-only to reviewizard. The app stores project paths and review data, including comments and snippets, on this machine. It works offline, runs no AI, and never uploads project or review data.</p>
            </section>
            <section className="app-help-section app-help-handoffs">
              <h3><IconArrowRight aria-hidden="true" />Optional handoffs</h3>
              <p>You choose whether to share a handoff and which tool receives it. The tool needs access to the repository and the reviewizard API from the same machine.</p>
              <ul className="app-help-modes">
                <li><IconFileSearch aria-hidden="true" /><span><strong>Review.</strong> The tool reads the code and posts findings through the API.</span></li>
                <li><IconTools aria-hidden="true" /><span><strong>Completion.</strong> The tool also needs permission to edit the code so it can fix and verify findings, then reply and resolve them through the API.</span></li>
              </ul>
              <HandoffGuide />
            </section>
            <section className="app-help-section app-help-start">
              <h3><IconFolderPlus aria-hidden="true" />Getting started</h3>
              <ol>
                <li>On <strong>Projects</strong>, choose <strong>Add Project</strong> and select a local Git folder.</li>
                <li>Choose <strong>Add review</strong> on the project card, set the scope, and select <strong>Save review</strong>. Open the saved review to inspect changes.</li>
              </ol>
            </section>
          </div>
        </Modal>
      )}
    </header>
  );
}

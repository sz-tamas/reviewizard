import { Moon, Sun } from "lucide-react";
import { Brand } from "./Brand";
import { setAppearance, themeOf, useTheme } from "../preferences/theme";

export function AppHeader() {
  const dark = themeOf(useTheme()).mermaid === "dark";

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
          aria-label="Light theme"
          aria-pressed={!dark}
          title={dark ? "Switch to light theme" : "Switch to dark theme"}
          onClick={() => setAppearance(dark ? "github-light" : "github-dark")}
        >
          {dark ? <Sun aria-hidden="true" /> : <Moon aria-hidden="true" />}
        </button>
      </div>
    </header>
  );
}

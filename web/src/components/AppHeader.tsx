import { Brand } from "./Brand";
export function AppHeader() {
  return (
    <header className="topbar app-header">
      <a className="header-brand" href="/" aria-label="reviewizard home">
        <Brand />
      </a>
      <div className="header-utilities">
        <span className="header-local"><span aria-hidden="true" className="header-local-dot" />Local (localhost)</span>
      </div>
    </header>
  );
}

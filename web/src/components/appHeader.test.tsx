import { fireEvent, render, screen } from "@testing-library/react";
import { expect, test, vi } from "vitest";
import { LS } from "../preferences/storage";

test("the header switches appearance across repositories and restores it on reload", async () => {
  localStorage.setItem(LS.themeByRepo, JSON.stringify({ api: "darcula" }));
  const { AppHeader } = await import("./AppHeader");
  const { setThemeRepo } = await import("../preferences/theme");
  const { unmount } = render(<AppHeader />);

  const toggle = screen.getByRole("button", { name: "Light theme" });
  expect(toggle.getAttribute("aria-pressed")).toBe("false");
  fireEvent.click(toggle);
  expect(toggle.getAttribute("aria-pressed")).toBe("true");
  expect(document.documentElement.dataset.theme).toBe("github-light");
  expect(localStorage.getItem(LS.appearance)).toBe("github-light");

  unmount();
  setThemeRepo("api");
  expect(document.documentElement.dataset.theme).toBe("github-light");

  localStorage.setItem(LS.repo, "api");
  vi.resetModules();
  const reloaded = await import("../preferences/theme");
  expect(reloaded.getTheme()).toBe("github-light");
  reloaded.setAppearance("github-dark");
  expect(document.documentElement.classList.contains("dark")).toBe(true);
  expect(localStorage.getItem(LS.appearance)).toBe("github-dark");
  localStorage.clear();
});

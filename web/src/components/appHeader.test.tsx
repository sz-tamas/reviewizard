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

test("header help opens a named dialog and returns focus on Escape", async () => {
  const { AppHeader } = await import("./AppHeader");
  render(<AppHeader />);

  const help = screen.getByRole("button", { name: "Help" });
  help.focus();
  fireEvent.click(help);

  expect(screen.getByRole("dialog", { name: "About reviewizard" })).toBeTruthy();
  expect(screen.getByRole("heading", { name: "Getting started" })).toBeTruthy();
  const close = screen.getByRole("button", { name: "Close" });
  expect(document.activeElement).toBe(close);
  const readMore = screen.getByRole("button", { name: "Read more" });
  expect(screen.queryByRole("heading", { name: "Ask for a review" })).toBeNull();
  fireEvent.click(readMore);
  expect(readMore.getAttribute("aria-expanded")).toBe("true");
  expect(screen.getByRole("heading", { name: "Ask for a review" })).toBeTruthy();
  expect(screen.getByRole("heading", { name: "Address findings and complete the review" })).toBeTruthy();
  fireEvent.click(screen.getByRole("button", { name: "Read less" }));
  expect(readMore.getAttribute("aria-expanded")).toBe("false");
  fireEvent.keyDown(close, { key: "Escape" });

  expect(screen.queryByRole("dialog")).toBeNull();
  expect(document.activeElement).toBe(help);
});

import { expect, test } from "vitest";
import IconFile from "@tabler/icons-react/icons/IconFile";
import IconLock from "@tabler/icons-react/icons/IconLock";
import { fileTypeForPath } from "./FileTypeIcon";

test("every tracked file format in reviewizard has a specific icon", () => {
  const paths = [
    "LICENSE", ".gitattributes", ".gitignore", "styles.css", "index.html", "tool.js",
    "package.json", "Cargo.lock", "README.md", "start.mjs", "logo.png", "script.py",
    "script.pyc", "main.rs", "schema.sql", "mise.toml", "main.ts", "App.tsx",
    "notice.txt", "Inter.woff2", "process-compose.yaml",
  ];
  for (const path of paths) {
    expect(fileTypeForPath(path).icon, path).not.toBe(IconFile);
  }
});

test("specific filenames beat extensions, and classification ignores case and directories", () => {
  expect(fileTypeForPath("web/PNPM-LOCK.YAML").icon).toBe(IconLock);
  expect(fileTypeForPath("web/src/APP.TSX").language).toBe("TypeScript");
  expect(fileTypeForPath("some.rs/unknown").icon).toBe(IconFile);
  expect(fileTypeForPath("file.unknown").icon).toBe(IconFile);
});

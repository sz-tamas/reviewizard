import IconBinary from "@tabler/icons-react/icons/IconBinary";
import IconBraces from "@tabler/icons-react/icons/IconBraces";
import IconBrandCSharp from "@tabler/icons-react/icons/IconBrandCSharp";
import IconBrandCpp from "@tabler/icons-react/icons/IconBrandCpp";
import IconBrandCss3 from "@tabler/icons-react/icons/IconBrandCss3";
import IconBrandDocker from "@tabler/icons-react/icons/IconBrandDocker";
import IconBrandGit from "@tabler/icons-react/icons/IconBrandGit";
import IconBrandGolang from "@tabler/icons-react/icons/IconBrandGolang";
import IconBrandHtml5 from "@tabler/icons-react/icons/IconBrandHtml5";
import IconBrandJavascript from "@tabler/icons-react/icons/IconBrandJavascript";
import IconBrandKotlin from "@tabler/icons-react/icons/IconBrandKotlin";
import IconBrandNodejs from "@tabler/icons-react/icons/IconBrandNodejs";
import IconBrandPhp from "@tabler/icons-react/icons/IconBrandPhp";
import IconBrandPython from "@tabler/icons-react/icons/IconBrandPython";
import IconBrandRust from "@tabler/icons-react/icons/IconBrandRust";
import IconBrandSass from "@tabler/icons-react/icons/IconBrandSass";
import IconBrandSvelte from "@tabler/icons-react/icons/IconBrandSvelte";
import IconBrandSwift from "@tabler/icons-react/icons/IconBrandSwift";
import IconBrandTypescript from "@tabler/icons-react/icons/IconBrandTypescript";
import IconBrandVue from "@tabler/icons-react/icons/IconBrandVue";
import IconCode from "@tabler/icons-react/icons/IconCode";
import IconCoffee from "@tabler/icons-react/icons/IconCoffee";
import IconDatabase from "@tabler/icons-react/icons/IconDatabase";
import IconDiamond from "@tabler/icons-react/icons/IconDiamond";
import IconFile from "@tabler/icons-react/icons/IconFile";
import IconFileSettings from "@tabler/icons-react/icons/IconFileSettings";
import IconFileText from "@tabler/icons-react/icons/IconFileText";
import IconFileTypeCsv from "@tabler/icons-react/icons/IconFileTypeCsv";
import IconFileTypeJsx from "@tabler/icons-react/icons/IconFileTypeJsx";
import IconFileTypePdf from "@tabler/icons-react/icons/IconFileTypePdf";
import IconFileTypeTsx from "@tabler/icons-react/icons/IconFileTypeTsx";
import IconFileZip from "@tabler/icons-react/icons/IconFileZip";
import IconLetterC from "@tabler/icons-react/icons/IconLetterC";
import IconLetterR from "@tabler/icons-react/icons/IconLetterR";
import IconLicense from "@tabler/icons-react/icons/IconLicense";
import IconLock from "@tabler/icons-react/icons/IconLock";
import IconMarkdown from "@tabler/icons-react/icons/IconMarkdown";
import IconMovie from "@tabler/icons-react/icons/IconMovie";
import IconMusic from "@tabler/icons-react/icons/IconMusic";
import IconPhoto from "@tabler/icons-react/icons/IconPhoto";
import IconTerminal2 from "@tabler/icons-react/icons/IconTerminal2";
import IconTypography from "@tabler/icons-react/icons/IconTypography";

const languages = new Map([
  ["Rust", IconBrandRust],
  ["TypeScript", IconBrandTypescript],
  ["JavaScript", IconBrandJavascript],
  ["Node.js", IconBrandNodejs],
  ["Python", IconBrandPython],
  ["Go", IconBrandGolang],
  ["Ruby", IconDiamond],
  ["Java", IconCoffee],
  ["C", IconLetterC],
  ["C#", IconBrandCSharp],
  ["C++", IconBrandCpp],
  ["Kotlin", IconBrandKotlin],
  ["Swift", IconBrandSwift],
  ["PHP", IconBrandPhp],
  ["R", IconLetterR],
]);

type Icon = typeof IconFile;

const filenames = new Map([
  [".gitattributes", IconBrandGit],
  [".gitignore", IconBrandGit],
  [".gitmodules", IconBrandGit],
  [".gitconfig", IconBrandGit],
  ["dockerfile", IconBrandDocker],
  ["containerfile", IconBrandDocker],
  ["makefile", IconFileSettings],
  ["gemfile", IconDiamond],
  ["rakefile", IconDiamond],
  ["procfile", IconFileSettings],
  ["cargo.lock", IconLock],
  ["pnpm-lock.yaml", IconLock],
  ["package-lock.json", IconLock],
  ["bun.lockb", IconLock],
  ["go.sum", IconLock],
]);

interface FileType {
  icon: Icon;
  language?: string;
}

const extensions = new Map<string, FileType>();
const fileTypes: [string, Icon, string?][] = [
  ["rs", IconBrandRust, "Rust"],
  ["ts mts cts", IconBrandTypescript, "TypeScript"],
  ["tsx", IconFileTypeTsx, "TypeScript"],
  ["js mjs cjs", IconBrandJavascript, "JavaScript"],
  ["jsx", IconFileTypeJsx, "JavaScript"],
  ["py pyi pyw pyc", IconBrandPython, "Python"],
  ["go", IconBrandGolang, "Go"],
  ["rb erb", IconDiamond, "Ruby"],
  ["java class jar", IconCoffee, "Java"],
  ["c h", IconLetterC, "C"],
  ["cpp cc cxx hpp hh hxx", IconBrandCpp, "C++"],
  ["cs csproj", IconBrandCSharp, "C#"],
  ["kt kts", IconBrandKotlin, "Kotlin"],
  ["swift", IconBrandSwift, "Swift"],
  ["php phtml", IconBrandPhp, "PHP"],
  ["r rmd", IconLetterR, "R"],
  ["html htm", IconBrandHtml5],
  ["css", IconBrandCss3],
  ["scss sass", IconBrandSass],
  ["vue", IconBrandVue],
  ["svelte", IconBrandSvelte],
  ["sh bash zsh fish ps1 bat cmd", IconTerminal2],
  ["lua dart ex exs erl hs clj scala zig nim fs fsx vb pl pm", IconCode],
  ["sql db sqlite sqlite3", IconDatabase],
  ["json jsonc json5", IconBraces],
  ["toml yaml yml ini conf cfg env xml props gradle mod", IconFileSettings],
  ["md markdown mdx", IconMarkdown],
  ["txt rst adoc log", IconFileText],
  ["csv tsv", IconFileTypeCsv],
  ["pdf", IconFileTypePdf],
  ["png jpg jpeg gif webp svg ico bmp avif tiff", IconPhoto],
  ["woff woff2 ttf otf", IconTypography],
  ["lock", IconLock],
  ["zip gz tgz tar bz2 xz rar 7z", IconFileZip],
  ["wasm exe dll so dylib bin", IconBinary],
  ["mp3 wav ogg flac m4a", IconMusic],
  ["mp4 webm mov mkv", IconMovie],
];
for (const [suffixes, icon, language] of fileTypes) {
  for (const suffix of suffixes.split(" ")) extensions.set(suffix, { icon, language });
}

export function fileTypeForPath(path: string): FileType {
  const name = path.slice(path.lastIndexOf("/") + 1).toLowerCase();
  const namedIcon = filenames.get(name);
  if (namedIcon) return { icon: namedIcon };
  if (/^(license|licence|copying)(\.|$)/.test(name)) return { icon: IconLicense };
  if (/^(dockerfile|containerfile)\./.test(name)) return { icon: IconBrandDocker };
  if (/^\.env(\.|$)/.test(name)) return { icon: IconFileSettings };
  const dot = name.lastIndexOf(".");
  return (dot >= 0 ? extensions.get(name.slice(dot + 1)) : undefined) ?? { icon: IconFile };
}

export function FileTypeIcon({ path }: { path: string }) {
  const { icon: Glyph, language } = fileTypeForPath(path);
  return <Glyph className="file-type-icon" data-language={language} aria-hidden="true" focusable="false" />;
}

export function LanguageIcon({ name }: { name: string }) {
  const Glyph = languages.get(name) ?? IconCode;
  return <Glyph aria-hidden="true" focusable="false" />;
}

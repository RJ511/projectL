import { execSync } from "node:child_process";

function listChangedFiles() {
  try {
    const output = execSync("git diff --name-only HEAD", {
      encoding: "utf8",
      stdio: ["ignore", "pipe", "ignore"],
    });
    return output
      .split(/\r?\n/)
      .map((line) => line.trim())
      .filter(Boolean);
  } catch {
    return [];
  }
}

const codePatterns = [
  /^src\//,
  /^src-tauri\//,
  /^package\.json$/,
  /^vite\.config\.js$/,
];

const docsPatterns = [
  /^docs\//,
  /^OLM_LOGIC_MAP\.md$/,
  /^README\.md$/,
  /^\.github\/copilot-instructions\.md$/,
];

const changed = listChangedFiles();
const hasCodeChanges = changed.some((file) =>
  codePatterns.some((pattern) => pattern.test(file)),
);
const hasDocsChanges = changed.some((file) =>
  docsPatterns.some((pattern) => pattern.test(file)),
);

if (hasCodeChanges && !hasDocsChanges) {
  console.error(
    "Docs sync check failed: code changed without docs updates. Update canonical docs before finishing.",
  );
  process.exit(1);
}

console.log("Docs sync check passed.");

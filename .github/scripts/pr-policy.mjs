import { execFileSync } from "node:child_process";
import { appendFileSync, readFileSync } from "node:fs";
import { pathToFileURL } from "node:url";

export const REQUIRED_SECTIONS = [
  "Summary",
  "Changes",
  "Related issue",
  "Validation",
  "Screenshots",
  "Change classification",
  "Checklist",
];

export const REQUIRED_CHECKLIST_ITEMS = [
  "I ran the repository checks required for the files I changed and reported the results above.",
  "I updated relevant documentation, or confirmed that no documentation update is needed.",
  "I added or updated tests for changed behavior, or explained above why tests are not needed.",
  "I did not commit secrets, credentials, generated build output, or local development artifacts.",
  "I preserved compatibility for saved projects, frontend/Tauri payloads, and public component interfaces where applicable.",
  "I updated `CHANGELOG.md` for a user-visible change, or selected internal-only above.",
];

const TITLE_PATTERN = /^(feat|fix|refactor|perf|docs|test|build|ci|chore|revert)\([a-z0-9-]+\): [a-z0-9][^\r\n]*[^.\s]$/;
const PLACEHOLDER_PATTERN = /<!--|<reason>|explain the|list the|attach before/i;

function escapeRegExp(value) {
  return value.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
}

export function parseSections(body) {
  const sections = new Map();
  const matches = [...body.matchAll(/^##\s+(.+?)\s*$/gm)];
  for (let index = 0; index < matches.length; index += 1) {
    const match = matches[index];
    const start = match.index + match[0].length;
    const end = matches[index + 1]?.index ?? body.length;
    sections.set(match[1].trim().toLowerCase(), body.slice(start, end).trim());
  }
  return sections;
}

function meaningfulSection(value) {
  const withoutComments = value.replace(/<!--[\s\S]*?-->/g, "").trim();
  return withoutComments.length > 0 && !PLACEHOLDER_PATTERN.test(withoutComments);
}

function checkboxState(body, label) {
  const pattern = new RegExp(`^- \\[([ xX])\\] ${escapeRegExp(label)}\\s*$`, "m");
  const match = body.match(pattern);
  return match ? match[1].toLowerCase() === "x" : null;
}

export function classifyPaths(files) {
  const matches = (pattern) => files.some((file) => pattern.test(file));
  const frontend = matches(/^(src\/.*\.(vue|ts|tsx|js|jsx|css)|index\.html|package\.json|pnpm-lock\.yaml|vite\.config\.ts|tsconfig(?:\.[^/]+)?\.json)$/);
  const rust = matches(/^src-tauri\/(?:.*\.rs|Cargo\.(?:toml|lock)|crates\/.*\/Cargo\.toml)$/);
  const mediaCore = matches(/^src-tauri\/crates\/linglux-media-core\//);
  const packaging = matches(/^src-tauri\/(?:tauri\.conf\.json|capabilities\/|icons\/|build\.rs)/);
  const ui = matches(/^(src\/.*\.vue|src\/style\.css|src\/components\/ui\/|index\.html)$/);
  return { frontend, rust, mediaCore, packaging, ui };
}

export function validatePullRequest({ event, changedFiles, trackedOutputFiles, changelog }) {
  const errors = [];
  const pullRequest = event.pull_request ?? {};
  const title = pullRequest.title ?? "";
  const body = pullRequest.body ?? "";
  const number = pullRequest.number ?? event.number;
  const repository = event.repository?.full_name ?? "wrx012/linglux";
  const paths = classifyPaths(changedFiles);

  if (title.length > 100 || !TITLE_PATTERN.test(title)) {
    errors.push("PR title must match `type(scope): imperative summary`, use an allowed lowercase type/scope, be at most 100 characters, and have no trailing period.");
  }

  const sections = parseSections(body);
  for (const heading of REQUIRED_SECTIONS) {
    const value = sections.get(heading.toLowerCase());
    if (!value || !meaningfulSection(value)) {
      errors.push(`Complete the \`${heading}\` section of the pull request template.`);
    }
  }

  const userVisible = checkboxState(body, "User-visible change");
  const internalOnly = checkboxState(body, "Internal-only change");
  if (userVisible === null || internalOnly === null || userVisible === internalOnly) {
    errors.push("Check exactly one change classification: `User-visible change` or `Internal-only change`.");
  }

  for (const item of REQUIRED_CHECKLIST_ITEMS) {
    if (checkboxState(body, item) !== true) {
      errors.push(`Complete the checklist item: ${item}`);
    }
  }

  const screenshots = sections.get("screenshots") ?? "";
  const screenshotText = screenshots.replace(/<!--[\s\S]*?-->/g, "").trim();
  if (paths.ui && (/^n\/?a\s*$/i.test(screenshotText) || /^n\/?a\s*[—-]\s*<reason>\s*$/i.test(screenshotText))) {
    errors.push("UI changes require screenshots or `N/A — <specific reason>` in the Screenshots section.");
  }

  if (userVisible === true) {
    if (!changedFiles.includes("CHANGELOG.md")) {
      errors.push("User-visible changes must modify `CHANGELOG.md` under `Unreleased`.");
    } else {
      const entryPattern = new RegExp(`\\(\\[PR #${number}\\]\\)`);
      const linkPattern = new RegExp(`^\\[PR #${number}\\]: https://github\\.com/${escapeRegExp(repository)}/pull/${number}\\s*$`, "m");
      const unreleased = changelog.match(/## \[Unreleased\]([\s\S]*?)(?=\n## \[|$)/)?.[1] ?? "";
      if (!entryPattern.test(unreleased) || !linkPattern.test(changelog)) {
        errors.push(`The Unreleased changelog entry must reference \`[PR #${number}]\` and define its verified repository URL.`);
      }
    }
  }

  if (trackedOutputFiles.length > 0) {
    errors.push(`Development artifacts under \`output/\` must not be tracked: ${trackedOutputFiles.join(", ")}`);
  }

  return { errors, paths };
}

function gitLines(args) {
  const output = execFileSync("git", args, { encoding: "utf8" }).trim();
  return output ? output.split("\n").filter(Boolean) : [];
}

function setOutput(name, value) {
  if (!process.env.GITHUB_OUTPUT) return;
  appendFileSync(process.env.GITHUB_OUTPUT, `${name}=${String(value)}\n`);
}

export async function main(argv = process.argv.slice(2)) {
  const values = new Map();
  for (let index = 0; index < argv.length; index += 2) values.set(argv[index], argv[index + 1]);
  const eventPath = values.get("--event");
  const base = values.get("--base");
  const head = values.get("--head");
  if (!eventPath || !base || !head) throw new Error("Usage: pr-policy.mjs --event <path> --base <sha> --head <sha>");

  const event = JSON.parse(readFileSync(eventPath, "utf8"));
  const changedFiles = gitLines(["diff", "--name-only", `${base}...${head}`]);
  const trackedOutputFiles = gitLines(["ls-files", "output"]);
  let changelog = "";
  try {
    changelog = readFileSync("CHANGELOG.md", "utf8");
  } catch {
    // Validation reports the missing changelog when it is required.
  }

  const result = validatePullRequest({ event, changedFiles, trackedOutputFiles, changelog });
  for (const [name, value] of Object.entries(result.paths)) setOutput(name, value);

  if (event.pull_request?.draft) {
    console.log("Draft PR: policy findings are advisory until the PR is marked ready for review.");
    for (const error of result.errors) console.log(`::warning::${error}`);
    return;
  }
  if (result.errors.length > 0) {
    for (const error of result.errors) console.error(`::error::${error}`);
    process.exitCode = 1;
    return;
  }
  console.log("Pull request policy validation passed.");
}

if (import.meta.url === pathToFileURL(process.argv[1] ?? "").href) {
  await main();
}
